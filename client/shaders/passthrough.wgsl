struct VertexIn {
    @location(0 /*position_location*/) position: vec4<f32>,
    @location(1) color : vec4<f32>,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_passthrough( in: VertexIn ) -> VertexOut {
    return VertexOut( in.position, in.color );
}

@fragment
fn ps_passthrough( in: VertexOut ) -> @location(0) vec4<f32> {
    return in.color;
}