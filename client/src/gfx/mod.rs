pub mod debugdraw;
pub mod init;
pub mod model;
pub mod renderpass;
pub mod rendertarget;
pub mod shader;
pub mod shader_attributes;

use std::sync::{Arc, RwLock};

pub use init::init;

use rendertarget::RenderTargetCache;
use shader::ShaderCache;

pub struct State {
    surface: wgpu::Surface,
    current_surface_texture: Option<wgpu::SurfaceTexture>,
    device: Arc<wgpu::Device>,
    queue: wgpu::Queue,
    surface_config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,

    shader_cache: RwLock<ShaderCache>,
    rendertarget_cache: RenderTargetCache,
}

pub fn do_frame<'a, T: 'a>(
    state: &mut State,
    frame_func: T,
) -> Result<(), Box<dyn std::error::Error>>
where
    T: FnOnce(&mut State, &mut wgpu::CommandEncoder),
{
    let shader_cache_frame_status = state.shader_cache.write().unwrap().do_frame();
    if shader_cache_frame_status
        .contains(shader::ShaderCacheFrameStatus::SKIP_FRAME_FOR_SHADER_PROCESSING)
    {
        return Ok(());
    }

    state.current_surface_texture = Some(state.surface.get_current_texture()?.into());

    let mut encoder = state
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

    frame_func(state, &mut encoder);

    let command_buffer = encoder.finish();
    state.queue.submit(std::iter::once(command_buffer));
    state.current_surface_texture.take().unwrap().present();

    Ok(())
}
