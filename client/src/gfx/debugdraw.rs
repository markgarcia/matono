use super::{
    model::Model,
    renderpass::{RenderPass, RenderPassFrameState},
    rendertarget::{ColorRenderTargetKey, DepthRenderTargetKey},
    shader::{PixelShaderKey, VertexShaderKey},
};

pub struct RenderTest {
    render_pass: RenderPass,
    vertex_buffer: Option<wgpu::Buffer>,
    model: Model,
}

impl Default for RenderTest {
    fn default() -> Self {
        Self {
            render_pass: RenderPass::new("RenderTest"),
            vertex_buffer: Default::default(),
            model: Default::default(),
        }
    }
}

impl RenderTest {
    pub async fn prep(&mut self, state: &super::State) -> Result<(), Box<dyn std::error::Error>> {
        self.render_pass.vs = VertexShaderKey::PassthroughVS;
        self.render_pass.ps = PixelShaderKey::PassthroughPS;
        //self.render_pass.vs = VertexShaderKey::GltfVS;
        //self.render_pass.ps = PixelShaderKey::GltfPS;
        self.render_pass
            .color_render_targets
            .push(ColorRenderTargetKey::Window);
        self.render_pass.depth_render_target = DepthRenderTargetKey::Window;

        self.vertex_buffer = Some(wgpu::util::DeviceExt::create_buffer_init(
            state.device.as_ref(),
            &wgpu::util::BufferInitDescriptor {
                label: Some("Vertex buffer"),
                contents: bytemuck::cast_slice(&[
                    [
                        0.0_f32, 0.5_f32, 0.0_f32, 1.0_f32, 0.0_f32, 0.0_f32, 1.0_f32,
                    ],
                    [
                        -0.5_f32, -0.5_f32, 0.0_f32, 0.0_f32, 1.0_f32, 0.0_f32, 1.0_f32,
                    ],
                    [
                        0.5_f32, -0.5_f32, 0.0_f32, 0.0_f32, 0.0_f32, 1.0_f32, 1.0_f32,
                    ],
                ]),
                usage: wgpu::BufferUsages::VERTEX,
            },
        ));

        self.model
            .from_gltf("data/testmodels/Box.glb", state)
            .await?;

        Ok(())
    }

    pub fn frame(&mut self, state: &super::State, encoder: &mut wgpu::CommandEncoder) {
        let mut render_pass_frame_state = RenderPassFrameState::new();
        let mut render_pass =
            self.render_pass
                .begin_frame(&mut render_pass_frame_state, state, encoder);

        render_pass.set_vertex_buffer(0, self.vertex_buffer.as_ref().unwrap().slice(..));
        //render_pass.set_vertex_buffer(0, self.model.vertex_buffer.as_ref().unwrap().slice(..));
        render_pass.set_index_buffer(
            self.model.index_buffer.as_ref().unwrap().slice(..),
            wgpu::IndexFormat::Uint32,
        );
        render_pass.draw(0..3, 0..1);
    }
}
