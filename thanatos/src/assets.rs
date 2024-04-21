use std::path::Path;

use anyhow::Result;
use glam::{Vec3, Vec4};
use gltf::Glb;

use crate::renderer::{Renderer, Vertex};

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
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Material {
    pub colour: Vec4,
}

#[derive(Clone, Copy, Debug)]
pub struct MeshId(usize);
#[derive(Clone, Copy, Debug)]
pub struct MaterialId(usize);

#[derive(Default)]
pub struct Manager {
    meshes: Vec<Mesh>,
    materials: Vec<Material>,
}

impl Manager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_mesh(&mut self, mesh: Mesh) -> MeshId {
        self.meshes.push(mesh);
        MeshId(self.meshes.len() - 1)
    }

    pub fn get_mesh(&self, id: MeshId) -> Option<&Mesh> {
        self.meshes.get(id.0)
    }

    pub fn add_material(&mut self, material: Material) -> MaterialId {
        self.materials.push(material);
        MaterialId(self.materials.len() - 1)
    }

    pub fn get_material(&self, id: MaterialId) -> Option<&Material> {
        self.materials.get(id.0)
    }
}
