use bitflags::bitflags;
use std::{
    collections::HashMap,
    future::Future,
    hash::Hash,
    sync::{
        atomic::{AtomicU32, Ordering},
        mpsc, Arc, Condvar, Mutex,
    },
    thread::JoinHandle,
};

use crate::data;

use super::shader_attributes::find_vertex_attributes;

#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug, Default)]
pub enum VertexShaderKey {
    #[default]
    Invalid,
    PassthroughVS,
    GltfVS,
    Other(u32),
}

#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug, Default)]
pub enum PixelShaderKey {
    #[default]
    Invalid,
    PassthroughPS,
    GltfPS,
    Other(u32),
}

#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
pub enum ShaderKey {
    VS(VertexShaderKey),
    PS(PixelShaderKey),
}

pub struct ShaderRef<'a> {
    pub module: owning_ref::RwLockReadGuardRef<'a, ShaderCache, wgpu::ShaderModule>,
    pub entrypoint: owning_ref::RwLockReadGuardRef<'a, ShaderCache, str>,
}

struct ShaderDetails {
    module_handle: ShaderModuleHandle,
    entrypoint: String,
}

struct VertexShaderDetails {
    stride: u64,
    attributes: Vec<wgpu::VertexAttribute>,
    buffer_layout: Vec<wgpu::VertexBufferLayout<'static>>,
}

pub enum ShaderEntrypoints<'a> {
    VS((VertexShaderKey, &'a str)),
    PS((PixelShaderKey, &'a str)),
}

#[derive(Copy, Clone, Hash, Eq, PartialEq)]
pub struct ShaderModuleHandle(u32);

pub enum ShaderModuleConstant {
    Bool(bool),
}

pub struct ShaderModuleDescriptor {
    path: String,
    constants: Vec<(String, ShaderModuleConstant)>,
}

bitflags! {
    pub struct ShaderCacheFrameStatus: u8 {
        const OK = 0;
        const SKIP_FRAME_FOR_SHADER_PROCESSING = 0x01;
    }
}

struct CompileShaderTask {
    handle: ShaderModuleHandle,
    descriptor: ShaderModuleDescriptor,
}

struct FenceTask {}

enum ShaderProcessorTask {
    CompileShader(CompileShaderTask),
    Fence(FenceTask),
}

struct ShaderProcessorFutureRunner {
    #[cfg(not(target_family = "wasm"))]
    tokio_rt: tokio::runtime::Runtime,
}

impl ShaderProcessorFutureRunner {
    #[cfg(not(target_family = "wasm"))]
    fn run<F: Future<Output = ()> + 'static + std::marker::Send>(&self, f: F) {
        self.tokio_rt.spawn(f);
    }

    #[cfg(target_family = "wasm")]
    fn run<F: Future<Output = ()> + 'static>(&self, f: F) {
        wasm_bindgen_futures::spawn_local(f);
    }
}

impl Default for ShaderProcessorFutureRunner {
    fn default() -> Self {
        Self {
            #[cfg(not(target_family = "wasm"))]
            tokio_rt: tokio::runtime::Runtime::new().unwrap(),
        }
    }
}

struct ShaderProcessor {
    device: Arc<wgpu::Device>,
    shaders_to_process_rx: mpsc::Receiver<ShaderProcessorTask>,
    processed_shaders: mpsc::Sender<(ShaderModuleHandle, wgpu::ShaderModule)>,
    fence_condvar: Arc<(Mutex<u32>, Condvar)>,
    compile_task_counter: Arc<AtomicU32>,
}

impl ShaderProcessor {
    async fn compile_shader(
        device: Arc<wgpu::Device>,
        shader_to_process: CompileShaderTask,
        processed_shaders: mpsc::Sender<(ShaderModuleHandle, wgpu::ShaderModule)>,
        fence_condvar: Arc<(Mutex<u32>, Condvar)>,
        compile_task_counter: Arc<AtomicU32>,
    ) {
        let path = shader_to_process.descriptor.path.as_str();

        let shader_buf = data::read_bytes(path).await.unwrap();

        let shader_source = {
            if path.ends_with(".wgsl") {
                wgpu::ShaderSource::Wgsl(std::borrow::Cow::from(
                    std::str::from_utf8(&shader_buf[..]).unwrap(),
                ))
            }
            /*else if path.ends_with(".spv") {
                let (prefix, shader_buf_u32, suffix) =
                    unsafe { shader_buf[..].align_to::<u32>() };
                if !prefix.is_empty() || !suffix.is_empty() {
                    Err(format!("Invalid spir-v file {}", path))?
                }

                wgpu::ShaderSource::SpirV(std::borrow::Cow::from(shader_buf_u32))
            }*/
            else {
                panic!("Unsupported shader format");
            }
        };

        let new_shader_module = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: Some(path),
            source: shader_source,
        });

        processed_shaders
            .send((shader_to_process.handle, new_shader_module))
            .unwrap();

        {
            let mut counter = fence_condvar.0.lock().unwrap();
            *counter = counter.checked_sub(1).unwrap();
            fence_condvar.1.notify_all();

            compile_task_counter.fetch_sub(1, Ordering::AcqRel);
        }
    }

    fn get_next_task(&self) -> Option<ShaderProcessorTask> {
        #[cfg(target_family = "wasm")]
        {
            self.shaders_to_process_rx.try_recv().ok()
        }

        #[cfg(not(target_family = "wasm"))]
        {
            self.shaders_to_process_rx.recv().ok()
        }
    }

    fn process(&self) {
        let current_thread_runner = ShaderProcessorFutureRunner::default();

        while let Some(shader_to_process) = self.get_next_task() {
            match shader_to_process {
                ShaderProcessorTask::CompileShader(compile_shader_task) => {
                    self.compile_task_counter.fetch_add(1, Ordering::AcqRel);

                    current_thread_runner.run(Self::compile_shader(
                        self.device.clone(),
                        compile_shader_task,
                        self.processed_shaders.clone(),
                        self.fence_condvar.clone(),
                        self.compile_task_counter.clone(),
                    ))
                }
                ShaderProcessorTask::Fence(_) => {
                    let mut counter = self.fence_condvar.0.lock().unwrap();
                    *counter = self.compile_task_counter.load(Ordering::Acquire);
                    self.fence_condvar.1.notify_all();
                }
            }
        }
    }
}

pub struct ShaderCache {
    frame_status: ShaderCacheFrameStatus,
    shader_module_handle_counter: AtomicU32,
    shaders: HashMap<ShaderKey, ShaderDetails>,
    shader_modules: HashMap<ShaderModuleHandle, wgpu::ShaderModule>,
    vertex_shader_details: HashMap<VertexShaderKey, VertexShaderDetails>,
    #[cfg(not(target_family = "wasm"))]
    shader_processor_thread: Option<JoinHandle<()>>,
    shader_processor_fence_condvar: Arc<(Mutex<u32>, Condvar)>,
    #[cfg(target_family = "wasm")]
    shader_processor: ShaderProcessor,
    shaders_to_process_tx: mpsc::Sender<ShaderProcessorTask>,
    processed_shaders: mpsc::Receiver<(ShaderModuleHandle, wgpu::ShaderModule)>,
}

impl ShaderCache {
    pub fn new(device: Arc<wgpu::Device>) -> ShaderCache {
        let (shaders_to_process_tx, shaders_to_process_rx) = mpsc::channel();
        let (processed_shaders_tx, processed_shaders_rx) = mpsc::channel();

        let shader_processor_fence_condvar = Arc::new((Mutex::new(0), Condvar::new()));

        let shader_processor = ShaderProcessor {
            device,
            shaders_to_process_rx: shaders_to_process_rx,
            processed_shaders: processed_shaders_tx,
            fence_condvar: shader_processor_fence_condvar.clone(),
            compile_task_counter: Arc::new(AtomicU32::new(0)),
        };

        ShaderCache {
            frame_status: ShaderCacheFrameStatus::OK,
            shader_module_handle_counter: AtomicU32::new(1),
            shaders: HashMap::new(),
            shader_modules: HashMap::new(),
            vertex_shader_details: HashMap::new(),
            #[cfg(not(target_family = "wasm"))]
            shader_processor_thread: Some(std::thread::spawn(move || shader_processor.process())),
            shader_processor_fence_condvar,
            #[cfg(target_family = "wasm")]
            shader_processor,
            shaders_to_process_tx: shaders_to_process_tx,
            processed_shaders: processed_shaders_rx,
        }
    }

    fn submit_shader_processor_task(&mut self, task: ShaderProcessorTask) {
        self.shaders_to_process_tx.send(task).unwrap();
    }

    pub fn receive_processed_shaders(&mut self) {
        {
            let counter = self.shader_processor_fence_condvar.0.lock().unwrap();
            if *counter == 0 {
                self.frame_status
                    .remove(ShaderCacheFrameStatus::SKIP_FRAME_FOR_SHADER_PROCESSING);
            }
        }

        while let Ok((shader_module_handle, shader_module)) = self.processed_shaders.try_recv() {
            self.shader_modules
                .insert(shader_module_handle, shader_module);
        }
    }

    pub fn wait_for_shader_processing(&mut self) {
        self.submit_shader_processor_task(ShaderProcessorTask::Fence(FenceTask {}));

        self.frame_status
            .insert(ShaderCacheFrameStatus::SKIP_FRAME_FOR_SHADER_PROCESSING);
    }

    pub fn do_frame(&mut self) -> ShaderCacheFrameStatus {
        #[cfg(target_family = "wasm")]
        {
            self.shader_processor.process();
        }

        self.receive_processed_shaders();
        return self.frame_status;
    }
}

impl super::State {
    pub fn is_shader_ready(&self, shader_key: ShaderKey) -> bool {
        let shader_cache = self.shader_cache.read().unwrap();
        shader_cache.shaders.get(&shader_key).is_some()
    }

    pub fn use_shader<'a>(&'a self, shader_key: ShaderKey) -> ShaderRef<'a> {
        let shader_cache = self.shader_cache.read().unwrap();
        let shader = shader_cache.shaders.get(&shader_key).unwrap();

        if !shader_cache
            .shader_modules
            .contains_key(&shader.module_handle)
        {
            todo!("Fallback to compatible proxy shader")
        }

        ShaderRef {
            module: owning_ref::RwLockReadGuardRef::new(self.shader_cache.read().unwrap()).map(
                |shader_cache| {
                    shader_cache
                        .shader_modules
                        .get(&shader.module_handle)
                        .unwrap()
                },
            ),
            entrypoint: owning_ref::RwLockReadGuardRef::new(self.shader_cache.read().unwrap()).map(
                |shader_cache| {
                    shader_cache
                        .shaders
                        .get(&shader_key)
                        .unwrap()
                        .entrypoint
                        .as_str()
                },
            ),
        }
    }

    pub fn find_vertex_buffer_layout<'a>(
        &'a self,
        vs: VertexShaderKey,
    ) -> owning_ref::RwLockReadGuardRef<'a, ShaderCache, [wgpu::VertexBufferLayout]> {
        owning_ref::RwLockReadGuardRef::new(self.shader_cache.read().unwrap()).map(|shader_cache| {
            shader_cache
                .vertex_shader_details
                .get(&vs)
                .unwrap()
                .buffer_layout
                .as_slice()
        })
    }

    pub fn find_vertex_shader_stride(&self, vs: VertexShaderKey) -> u64 {
        self.shader_cache
            .read()
            .unwrap()
            .vertex_shader_details
            .get(&vs)
            .unwrap()
            .stride
    }

    pub fn add_shader_module(
        &mut self,
        descriptor: ShaderModuleDescriptor,
        entrypoints: &[ShaderEntrypoints<'_>],
    ) -> Result<ShaderModuleHandle, Box<dyn std::error::Error>> {
        let mut shader_cache = self.shader_cache.write().unwrap();

        let path = descriptor.path.clone();

        let handle = ShaderModuleHandle(
            shader_cache
                .shader_module_handle_counter
                .fetch_add(1, Ordering::Relaxed),
        );

        shader_cache.submit_shader_processor_task(ShaderProcessorTask::CompileShader(
            CompileShaderTask { handle, descriptor },
        ));

        for entrypoint in entrypoints {
            match entrypoint {
                ShaderEntrypoints::VS((key, entrypoint_func)) => {
                    let attributes = find_vertex_attributes(&path, &entrypoint_func);

                    let mut stride: u64 = 0;
                    for attribute in attributes.iter() {
                        stride += attribute.format.size();
                    }

                    let mut buffer_layout = Vec::new();
                    buffer_layout.push(wgpu::VertexBufferLayout {
                        array_stride: stride,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: attributes,
                    });

                    let vertex_shader_details = VertexShaderDetails {
                        stride: stride,
                        attributes: attributes.to_vec(),
                        buffer_layout,
                    };

                    shader_cache
                        .vertex_shader_details
                        .insert(*key, vertex_shader_details);

                    shader_cache.shaders.insert(
                        ShaderKey::VS(*key),
                        ShaderDetails {
                            module_handle: handle,
                            entrypoint: entrypoint_func.to_string(),
                        },
                    );
                }
                ShaderEntrypoints::PS((key, entrypoint_func)) => {
                    shader_cache.shaders.insert(
                        ShaderKey::PS(*key),
                        ShaderDetails {
                            module_handle: handle,
                            entrypoint: entrypoint_func.to_string(),
                        },
                    );
                }
            }
        }

        Ok(handle)
    }

    pub async fn init_shader_cache(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        use PixelShaderKey::*;
        use ShaderEntrypoints::*;
        use VertexShaderKey::*;

        self.add_shader_module(
            ShaderModuleDescriptor {
                path: "data/shaders/passthrough.wgsl".to_string(),
                constants: vec![],
            },
            &[
                VS((PassthroughVS, "vs_passthrough")),
                PS((PassthroughPS, "ps_passthrough")),
            ],
        )?;

        self.add_shader_module(
            ShaderModuleDescriptor {
                path: "data/shaders/gltf.wgsl".to_string(),
                constants: vec![],
            },
            &[VS((GltfVS, "vs")), PS((GltfPS, "ps"))],
        )?;

        self.shader_cache
            .write()
            .unwrap()
            .wait_for_shader_processing();

        Ok(())
    }
}
