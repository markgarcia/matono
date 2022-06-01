#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug, Default)]
pub enum ColorRenderTargetKey {
    #[default]
    Invalid,
    Window,
    Other(u32),
}

#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug, Default)]
pub enum DepthRenderTargetKey {
    #[default]
    Invalid,
    Window,
    Other(u32),
}

#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
pub enum RenderTargetKey {
    Color(ColorRenderTargetKey),
    Depth(DepthRenderTargetKey),
}

pub struct RenderTargetCache {
    depth_buffer_texture: Option<wgpu::Texture>,
}

impl RenderTargetCache {
    pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
}

impl Default for RenderTargetCache {
    fn default() -> Self {
        RenderTargetCache {
            depth_buffer_texture: None,
        }
    }
}

impl super::State {
    pub fn find_render_target(&self, key: RenderTargetKey) -> wgpu::TextureView {
        match key {
            RenderTargetKey::Color(color_key) => match color_key {
                ColorRenderTargetKey::Invalid => panic!("Invalid render target"),
                ColorRenderTargetKey::Window => {
                    let render_target = self.current_surface_texture.as_ref().unwrap();
                    render_target
                        .texture
                        .create_view(&wgpu::TextureViewDescriptor::default())
                }
                ColorRenderTargetKey::Other(_) => todo!(),
            },
            RenderTargetKey::Depth(depth_key) => match depth_key {
                DepthRenderTargetKey::Invalid => panic!("Invalid render target"),
                DepthRenderTargetKey::Window => self
                    .rendertarget_cache
                    .depth_buffer_texture
                    .as_ref()
                    .unwrap()
                    .create_view(&wgpu::TextureViewDescriptor::default()),
                DepthRenderTargetKey::Other(_) => todo!(),
            },
        }
    }

    pub fn find_color_render_target(&self, key: ColorRenderTargetKey) -> wgpu::TextureView {
        self.find_render_target(RenderTargetKey::Color(key))
    }

    pub fn find_depth_render_target(&self, key: DepthRenderTargetKey) -> wgpu::TextureView {
        self.find_render_target(RenderTargetKey::Depth(key))
    }

    pub fn find_color_render_target_format(
        &self,
        key: ColorRenderTargetKey,
    ) -> wgpu::TextureFormat {
        match key {
            ColorRenderTargetKey::Invalid => todo!(),
            ColorRenderTargetKey::Window => self.surface_config.format,
            ColorRenderTargetKey::Other(_) => todo!(),
        }
    }

    pub fn find_depth_render_target_format(
        &self,
        key: DepthRenderTargetKey,
    ) -> wgpu::TextureFormat {
        match key {
            DepthRenderTargetKey::Invalid => todo!(),
            DepthRenderTargetKey::Window => RenderTargetCache::DEPTH_FORMAT,
            DepthRenderTargetKey::Other(_) => todo!(),
        }
    }

    pub fn init_builtin_render_targets(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.rendertarget_cache.depth_buffer_texture =
            Some(self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Depth texture"),
                size: wgpu::Extent3d {
                    width: self.size.width,
                    height: self.size.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: RenderTargetCache::DEPTH_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
            }));
        Ok(())
    }
}
