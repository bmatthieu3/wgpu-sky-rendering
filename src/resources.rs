trait ResourceDesc {}

use crate::render::{Mesh, MeshDesc};
impl ResourceDesc for MeshDesc {}

use std::path::Path;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
pub trait Loader {
    type DescType: ResourceDesc;
    fn load<P: AsRef<Path>>(&mut self, p: P, name: &'static str) -> Result<Self::DescType>;
}

use crate::vertex::WorldSpaceVertex;
pub struct MeshLoader {
    pub vertices: Vec<WorldSpaceVertex>,
    pub indices: Vec<u32>,
}

impl MeshLoader {
    pub fn new() -> Self {
        Self {
            vertices: vec![],
            indices: vec![]
        }
    }
}

use crate::render::MeshLabel;
extern crate gltf;
impl Loader for MeshLoader {
    type DescType = MeshDesc;
    fn load<P: AsRef<Path>>(&mut self, p: P, name: &'static str) -> Result<Self::DescType> {
        let (gltf, buffers, _) = gltf::import(&p)
            .map_err(|e| Box::new(e))?;

        let mut vertices = vec![];
        let mut indices = vec![];
        for mesh in gltf.meshes() {
            println!("Mesh #{}", mesh.index());
            for primitive in mesh.primitives() {
                println!("- Primitive #{}", primitive.index());
                let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));
                let (iter_vertex, iter_normals) = (reader.read_positions().ok_or("sfsd")?, reader.read_normals().ok_or("sdfs")?);

                let vertices_iter = iter_vertex.zip(iter_normals)
                    .map(|(vertex_position, vertex_normal)| {
                        WorldSpaceVertex {
                            position: vertex_position,
                            normals: vertex_normal,
                        }
                    });
                vertices.extend(vertices_iter);

                let iter_indices = reader.read_indices().ok_or("sdfsd")?;
                indices.extend(iter_indices.into_u32());
            }
        }

        let start_idx = self.indices.len() as u32;
        let num_indices = indices.len();
        let base_vertex_idx = self.vertices.len() as i32;

        self.vertices.extend(vertices);
        self.indices.extend(indices);

        Ok(
            MeshDesc {
                start_idx,
                num_indices,
                base_vertex_idx,
                ty: MeshLabel::Object {
                    path: p.as_ref().to_str().unwrap().to_string(),
                    name
                }
            }
        )
    }
}