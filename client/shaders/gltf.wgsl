
struct VertexIn {
    @location(0 /*position_location*/) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) tangent: vec4<f32>,
    @location(3) tex_coord_0: vec2<f32>,
    @location(4) tex_coord_1: vec2<f32>,
    @location(5) color_0: vec4<f32>,
    @location(6) joint_0: vec4<u32>,
    @location(7) weight_0: vec4<f32>,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

/*struct WorldParams {

}

@group(0) @binding(0)
var<uniform> world_params: WorldParams;

struct ModelParams {

}

@group(0) @binding(1)
var<uniform> model_params: ModelParams;*/


@vertex
fn vs( in: VertexIn ) -> VertexOut {
    var out = VertexOut();
    out.position = vec4<f32>(in.position.x, in.position.y, in.position.z, 0.0);
    out.color = in.color_0;
    return out;
}

@fragment
fn ps( in: VertexOut ) -> @location(0) vec4<f32> {
    return in.color;
}