use std::{collections::HashMap, path::Path};

use anyhow::Result;
use glam::{Vec3, Vec4};
use gltf::Glb;
use serde::{Deserialize, Serialize};

use crate::renderer::Vertex;

pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub num_indices: u32,
}

impl Mesh {
    pub fn load<T: AsRef<Path>>(path: T) -> Result<Self> {
        let model = Glb::load(&std::fs::read(path).unwrap()).unwrap();

        let positions: Vec<Vec3> = bytemuck::cast_slice::<u8, f32>(
            &model.gltf.meshes[0].primitives[0]
                .get_attribute_data(&model, "POSITION")
                .unwrap(),
        )
        .chunks(3)
        .map(Vec3::from_slice)
        .collect();

        let normals: Vec<Vec3> = bytemuck::cast_slice::<u8, f32>(
            &model.gltf.meshes[0].primitives[0]
                .get_attribute_data(&model, "NORMAL")
                .unwrap(),
        )
        .chunks(3)
        .map(Vec3::from_slice)
        .collect();

        let vertices: Vec<Vertex> = positions
            .into_iter()
            .zip(normals)
            .map(|(position, normal)| Vertex { position, normal })
            .collect();

        let indices: Vec<u32> = model.gltf.meshes[0].primitives[0]
            .get_indices_data(&model)
            .unwrap();

        Ok(Mesh {
            vertices,
            num_indices: indices.len() as u32,
            indices,
        })
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable, Serialize, Deserialize)]
pub struct Material {
    pub colour: Vec4,
}
impl Material {
    pub fn debug_material() -> Self {
        return Material {
            colour: Vec4::new(1.0, 0.0, 0.95, 1.0),
        };
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeshId(pub String);

impl AsRef<Path> for MeshId {
    fn as_ref(&self) -> &Path {
        self.0.as_ref()
    }
}

#[derive(Default)]
pub struct MeshCache(HashMap<MeshId, Mesh>);

impl MeshCache {
    pub fn load<P: AsRef<Path>>(&mut self, path: P) -> Result<&Mesh> {
        let id = MeshId(path.as_ref().to_str().unwrap().to_owned());
        if self.0.get(&id).is_none() {
            self.0.insert(id.clone(), Mesh::load(path)?);
        }
        Ok(self.0.get(&id).unwrap())
    }
}
