pub fn find_vertex_attributes(
    shader_path: &str,
    vs_entrypoint: &str,
) -> &'static [wgpu::VertexAttribute] {
    match (shader_path, vs_entrypoint) {
        ("data/shaders/passthrough.wgsl", "vs_passthrough") => {
            const ATTRIBUTES: &'static [wgpu::VertexAttribute] =
                &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x4];
            ATTRIBUTES
        }
        ("data/shaders/gltf.wgsl", "vs") => {
            const ATTRIBUTES: &'static [wgpu::VertexAttribute] = &wgpu::vertex_attr_array![
                0 => Float32x3,
                1 => Float32x3,
                2 => Float32x4,
                3 => Float32x2,
                4 => Float32x2,
                5 => Float32x4,
                6 => Uint32x4,
                7 => Float32x4
            ];
            ATTRIBUTES
        }
        _ => panic!(),
    }
}
