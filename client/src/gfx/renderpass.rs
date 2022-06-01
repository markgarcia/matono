use std::ops::Deref;

use wgpu::DepthStencilState;

use super::{
    rendertarget::{ColorRenderTargetKey, DepthRenderTargetKey},
    shader::{PixelShaderKey, ShaderKey, VertexShaderKey},
};

pub struct RenderPass {
    pub name: &'static str,
    pub vs: VertexShaderKey,
    pub ps: PixelShaderKey,
    pub color_render_targets: Vec<ColorRenderTargetKey>,
    pub depth_render_target: DepthRenderTargetKey,

    render_pipeline: Option<wgpu::RenderPipeline>,
}

impl RenderPass {
    pub fn new(name: &'static str) -> RenderPass {
        Self {
            name,
            vs: VertexShaderKey::Invalid,
            ps: PixelShaderKey::Invalid,
            color_render_targets: Vec::new(),
            depth_render_target: DepthRenderTargetKey::Invalid,
            render_pipeline: None,
        }
    }

    fn rebuild_pipeline(&mut self, state: &super::State) {
        let render_pipeline_layout =
            state
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some(&self.name),
                    bind_group_layouts: &[],
                    push_constant_ranges: &[],
                });

        let vs = state.use_shader(ShaderKey::VS(self.vs));
        let ps = state.use_shader(ShaderKey::PS(self.ps));

        let mut color_targets = Vec::with_capacity(self.color_render_targets.len());
        for target in &self.color_render_targets {
            color_targets.push(wgpu::ColorTargetState {
                format: state.find_color_render_target_format(*target),
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })
        }

        let depth_target = match self.depth_render_target {
            DepthRenderTargetKey::Invalid => None,
            key => Some(DepthStencilState {
                format: state.find_depth_render_target_format(key),
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
        };

        self.render_pipeline = Some(state.device.create_render_pipeline(
            &wgpu::RenderPipelineDescriptor {
                label: Some(&self.name),
                layout: Some(&render_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &vs.module,
                    entry_point: &vs.entrypoint,
                    buffers: state.find_vertex_buffer_layout(self.vs).deref(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &ps.module,
                    entry_point: &ps.entrypoint,
                    targets: &color_targets,
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: Some(wgpu::Face::Back),
                    polygon_mode: wgpu::PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil: depth_target,
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview: None,
            },
        ));
    }

    pub fn begin_frame<'a>(
        &'a mut self,
        frame_state: &'a mut RenderPassFrameState<'a>,
        state: &'a super::State,
        encoder: &'a mut wgpu::CommandEncoder,
    ) -> wgpu::RenderPass<'a> {
        if self.render_pipeline.is_none() {
            self.rebuild_pipeline(&state);
        }

        let mut render_pass = frame_state.build(self, state, encoder);
        render_pass.set_pipeline(&self.render_pipeline.as_ref().unwrap());
        render_pass
    }
}

pub struct RenderPassFrameState<'a> {
    color_texture_views: Vec<wgpu::TextureView>,
    color_attachments: Vec<wgpu::RenderPassColorAttachment<'a>>,
    depth_texture_view: Option<wgpu::TextureView>,
}

impl<'a> RenderPassFrameState<'a> {
    pub fn new() -> Self {
        RenderPassFrameState {
            color_texture_views: Vec::new(),
            color_attachments: Vec::new(),
            depth_texture_view: None,
        }
    }

    fn build(
        &'a mut self,
        render_pass: &RenderPass,
        state: &super::State,
        encoder: &'a mut wgpu::CommandEncoder,
    ) -> wgpu::RenderPass {
        self.color_texture_views = render_pass
            .color_render_targets
            .iter()
            .map(|target| state.find_color_render_target(*target))
            .collect();

        self.color_attachments = self
            .color_texture_views
            .iter()
            .map(|texture_view| wgpu::RenderPassColorAttachment {
                view: &texture_view,
                resolve_target: None,
                ops: wgpu::Operations::default(),
            })
            .collect();

        if render_pass.depth_render_target != DepthRenderTargetKey::Invalid {
            self.depth_texture_view =
                Some(state.find_depth_render_target(render_pass.depth_render_target));
        }

        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &self.color_attachments,
            depth_stencil_attachment: self.depth_texture_view.as_ref().map(|depth_texture_view| {
                wgpu::RenderPassDepthStencilAttachment {
                    view: &depth_texture_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: true,
                    }),
                    stencil_ops: None,
                }
            }),
        })
    }
}
