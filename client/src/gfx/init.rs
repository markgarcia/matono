use std::sync::{Arc, RwLock};

use winit::{dpi::PhysicalSize, window::WindowBuilder};

use super::{rendertarget::RenderTargetCache, shader::ShaderCache};

pub async fn init() -> Result<(super::State, super::super::WindowState), Box<dyn std::error::Error>>
{
    let event_loop = winit::event_loop::EventLoop::new();
    let window = WindowBuilder::new()
        .with_min_inner_size(PhysicalSize::<u32>::new(800, 600))
        .build(&event_loop)?;

    #[cfg(target_family = "wasm")]
    {
        use winit::platform::web::WindowExtWebSys;
        web_sys::window()
            .and_then(|win| win.document())
            .and_then(|doc| doc.body())
            .and_then(|body| {
                body.append_child(&web_sys::Element::from(window.canvas()))
                    .ok()
            })
            .expect("Couldn't append canvas to document body");
    }

    let size = window.inner_size();

    let instance = wgpu::Instance::new(wgpu::Backends::all());
    let surface = unsafe { instance.create_surface(&window) };

    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        })
        .await
        .ok_or("Graphics adapter not found")?;

    let limits = if cfg!(target_family = "wasm") {
        wgpu::Limits::downlevel_webgl2_defaults().using_resolution(adapter.limits())
    } else {
        wgpu::Limits::default()
    };

    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                features: wgpu::Features::empty(),
                limits: limits,
                label: None,
            },
            None,
        )
        .await?;

    let supported_formats = surface.get_supported_formats(&adapter);
    let format = supported_formats
        .first()
        .ok_or("Window surface format not compatible with graphics adapter")?;

    let surface_config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: *format,
        width: size.width,
        height: size.height,
        present_mode: wgpu::PresentMode::Fifo,
    };
    surface.configure(&device, &surface_config);

    let device_arc = Arc::new(device);
    let mut state = super::State {
        surface: surface,
        current_surface_texture: None,
        device: device_arc.clone(),
        queue: queue,
        surface_config: surface_config,
        size: size,
        shader_cache: RwLock::new(ShaderCache::new(device_arc.clone())),
        rendertarget_cache: RenderTargetCache::default(),
    };

    state.init_shader_cache().await?;
    state.init_builtin_render_targets()?;

    Ok((
        state,
        super::super::WindowState {
            window: Some(window),
            event_loop: Some(event_loop),
        },
    ))
}
