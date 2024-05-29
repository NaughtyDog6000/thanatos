mod assets;
mod camera;
mod collider;
mod colours;
mod craft;
mod equipment;
mod event;
mod gather;
mod interact;
mod inventory;
mod net;
mod player;
mod renderer;
mod transform;
mod window;

use crate::{camera::Camera, window::Window};
use anyhow::Result;
use assets::{Material, MeshCache, MeshId};
use collider::{Collider, ColliderKind};
use event::Event;
use gather::Gatherable;
use glam::{Vec3, Vec4};
use interact::Interactable;
use net::Connection;
use nyx::task::Proficiencies;
use player::Player;
use renderer::{RenderObject, Renderer};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tecs::prelude::*;
use tecs::scene::Scene;
use tecs::utils::{Clock, Name, State, Timer};
use transform::Transform;

#[derive(Archetype, Clone, Serialize, Deserialize)]
struct CopperOre {
    pub render: RenderObject,
    pub transform: Transform,
    pub gatherable: Gatherable,
    pub interactable: Interactable,
    pub name: Name,
}

impl CopperOre {
    pub fn new(world: &World) -> Result<Self> {
        Ok(CopperOre {
            render: RenderObject {
                mesh: MeshId(String::from("assets/meshes/copper_ore.glb")),
                material: Material {
                    colour: Vec4::new(1.0, 0.5, 0.0, 1.0),
                },
            },
            transform: Transform::IDENTITY,
            gatherable: Gatherable {
                collider: Collider {
                    kind: ColliderKind::Sphere(5.0),
                    position: Vec3::ZERO,
                },
                loot: 0,
                timer: Timer::new(Duration::from_secs(1)),
            },
            interactable: Interactable::new(&world, "Gather Copper Ore"),
            name: Name(String::from("Copper Ore")),
        })
    }

    pub fn with_transform(mut self, transform: Transform) -> Self {
        self.transform = transform;
        self
    }
}

pub type World = tecs::World<Event>;

fn main() -> Result<()> {
    pretty_env_logger::init();

    let window = Window::new();

    let renderer = Renderer::new(&window)?;
    let camera = Camera::new(&window);

    let world = World::new()
        .register::<Player>()
        .register::<CopperOre>()
        .with_resource(State::Running)
        .with_resource(Proficiencies::default())
        .with_resource(MeshCache::default())
        .with(Connection::add)
        .with(window.add())
        .with(renderer.add())
        .with(camera.add())
        .with(Clock::add)
        .with(inventory::add)
        .with(craft::add)
        .with(equipment::add)
        .with(interact::add)
        .with_handler(|world, event| match event {
            Event::Stop => {
                *world.get_mut::<State>().unwrap() = State::Stopped;
            }
            _ => (),
        })
        .with_ticker(|world| {
            let clock = world.get::<Clock>().unwrap();
            println!("FPS: {}", 1.0 / clock.delta.as_secs_f32());
        })
        .with_ticker(Player::tick)
        .with_ticker(gather::tick)
        .with(net::add);

    let mut transform = Transform::IDENTITY;
    transform.translation += Vec3::ZERO;

    /*
    world.spawn(Player {
        render: RenderObject {
            mesh: MeshId(String::from("assets/meshes/cube.glb")),
            material: Material { colour: Vec4::ONE },
        },
        transform,
        health: Health(100.0),
    });

    world.spawn(CopperOre::new(&world)?.with_transform(Transform {
        translation: Vec3::ONE,
        rotation: Quat::IDENTITY,
        scale: Vec3::new(3.0, 1.0, 2.0),
    }));

    let mut scene = Scene::default();
    scene.from_world(&world);

    let mut buffer: Vec<u8> = Vec::new();
    let mut serializer = serde_json::Serializer::pretty(&mut buffer);
    scene.save(&world, &mut serializer).unwrap();
    std::fs::write("assets/scenes/test.scene", buffer).unwrap();
    */

    let buffer = std::fs::read("assets/scenes/test.scene").unwrap();
    Scene::load(&world, &mut serde_json::Deserializer::from_slice(&buffer)).unwrap();

    loop {
        if let State::Stopped = *world.get::<State>().unwrap() {
            break;
        }
        world.tick();
    }

    Ok(())
}
