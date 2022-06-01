use crate::data;

#[derive(Default)]
pub struct Model {
    pub vertex_buffer: Option<wgpu::Buffer>,
    pub index_buffer: Option<wgpu::Buffer>,
    pub textures: Vec<wgpu::Texture>,
}

impl Model {
    pub async fn from_gltf(
        &mut self,
        path: &str,
        state: &super::State,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let buffer = data::read_bytes(path).await?;
        let (gltf_doc, gltf_buffers, gltf_images) = gltf::import_slice(buffer.as_slice())
            .map_err(|_| format!("Invalid gltf file: {}", path))?;

        let mesh = gltf_doc.meshes().nth(0).ok_or(format!(
            "gltf file must contain at least one mesh: {}",
            path
        ))?;

        let mesh_name = mesh.name().or(Some(&path)).unwrap();

        for primitive in mesh.primitives().nth(0) {
            let mode = primitive.mode();
            assert!(mode == gltf::json::mesh::Mode::Triangles);

            let reader = primitive.reader(|buffer| Some(&gltf_buffers[buffer.index()]));

            let indices: Vec<u32> = reader.read_indices().unwrap().into_u32().collect();

            self.index_buffer = Some(wgpu::util::DeviceExt::create_buffer_init(
                state.device.as_ref(),
                &wgpu::util::BufferInitDescriptor {
                    label: Some(&format!(
                        "Index buffer: {} [primitive {}]",
                        mesh_name,
                        primitive.index()
                    )),
                    contents: bytemuck::cast_slice(indices.as_slice()),
                    usage: wgpu::BufferUsages::INDEX,
                },
            ));

            let normals_default = dyn_iter::DynIter::new(std::iter::repeat([0_f32, 0_f32, 0_f32]));
            let tangents_default =
                dyn_iter::DynIter::new(std::iter::repeat([0_f32, 0_f32, 0_f32, 0_f32]));
            let tex_coords_0_default = dyn_iter::DynIter::new(std::iter::repeat([0_f32, 0_f32]));
            let tex_coords_1_default = dyn_iter::DynIter::new(std::iter::repeat([0_f32, 0_f32]));
            let colors_0_default =
                dyn_iter::DynIter::new(std::iter::repeat([1_f32, 1_f32, 1_f32, 1_f32]));
            let joints_0_default =
                dyn_iter::DynIter::new(std::iter::repeat([0_u16, 0_u16, 0_u16, 0_u16]));
            let weights_0_default =
                dyn_iter::DynIter::new(std::iter::repeat([0_f32, 0_f32, 0_f32, 0_f32]));

            let positions = reader.read_positions().unwrap();
            let normals = reader
                .read_normals()
                .map_or(normals_default, |iter| dyn_iter::DynIter::new(iter));
            let tangents = reader
                .read_tangents()
                .map_or(tangents_default, |iter| dyn_iter::DynIter::new(iter));
            let tex_coords_0 = reader
                .read_tex_coords(0)
                .map_or(tex_coords_0_default, |iter| {
                    dyn_iter::DynIter::new(iter.into_f32())
                });
            let tex_coords_1 = reader
                .read_tex_coords(0)
                .map_or(tex_coords_1_default, |iter| {
                    dyn_iter::DynIter::new(iter.into_f32())
                });
            let colors_0 = reader.read_colors(0).map_or(colors_0_default, |iter| {
                dyn_iter::DynIter::new(iter.into_rgba_f32())
            });
            let joints_0 = reader.read_joints(0).map_or(joints_0_default, |iter| {
                dyn_iter::DynIter::new(iter.into_u16())
            });
            let weights_0 = reader.read_weights(0).map_or(weights_0_default, |iter| {
                dyn_iter::DynIter::new(iter.into_f32())
            });

            let desired_capacity = positions.len()
                * state.find_vertex_shader_stride(crate::gfx::shader::VertexShaderKey::GltfVS)
                    as usize;

            let mut vertices_bytes = Vec::with_capacity(desired_capacity);
            for (position, normal, tangent, tex_coord_0, tex_coord_1, color_0, joint_0, weight_0) in
                itertools::multizip((
                    positions,
                    normals,
                    tangents,
                    tex_coords_0,
                    tex_coords_1,
                    colors_0,
                    joints_0,
                    weights_0,
                ))
            {
                vertices_bytes.extend_from_slice(bytemuck::bytes_of(&position));
                vertices_bytes.extend_from_slice(bytemuck::bytes_of(&normal));
                vertices_bytes.extend_from_slice(bytemuck::bytes_of(&tangent));
                vertices_bytes.extend_from_slice(bytemuck::bytes_of(&tex_coord_0));
                vertices_bytes.extend_from_slice(bytemuck::bytes_of(&tex_coord_1));
                vertices_bytes.extend_from_slice(bytemuck::bytes_of(&color_0));
                let joint_0_u32 = [
                    joint_0[0] as u32,
                    joint_0[1] as u32,
                    joint_0[2] as u32,
                    joint_0[3] as u32,
                ];
                vertices_bytes.extend_from_slice(bytemuck::bytes_of(&joint_0_u32));
                vertices_bytes.extend_from_slice(bytemuck::bytes_of(&weight_0));
            }

            assert!(vertices_bytes.len() == desired_capacity);

            self.vertex_buffer = Some(wgpu::util::DeviceExt::create_buffer_init(
                state.device.as_ref(),
                &wgpu::util::BufferInitDescriptor {
                    label: Some(&format!(
                        "Vertex buffer: {} [primitive {}]",
                        mesh_name,
                        primitive.index()
                    )),
                    contents: bytemuck::cast_slice(vertices_bytes.as_slice()),
                    usage: wgpu::BufferUsages::VERTEX,
                },
            ));
        }

        let model = Model::default();
        Ok(model)
    }
}
