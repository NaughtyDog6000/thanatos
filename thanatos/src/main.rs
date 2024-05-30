mod assets;
mod camera;
mod collider;
mod colours;
mod combat;
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
use std::time::{Duration, Instant};
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

#[derive(Archetype)]
struct Tree {
    pub render: RenderObject,
}

#[derive(Archetype, Clone, Serialize, Deserialize)]
struct TargetDummy {
    pub transform: Transform,
    pub render: RenderObject,
    pub defensive_stats: combat::CombatDefensive,
    pub collider: Collider,
}

#[derive(Archetype, Clone, Serialize, Deserialize)]
struct DebugSphere {
    pub transform: Transform,
    pub render: RenderObject,
}

// struct Timer {
//     start: Option<Instant>,
//     pub duration: Duration,
// }

// impl Timer {
//     pub fn new(duration: Duration) -> Self {
//         Self {
//             start: None,
//             duration,
//         }
//     }

//     pub fn start(&mut self) {
//         self.start = Some(Instant::now())
//     }

//     pub fn done(&self) -> bool {
//         self.start
//             .map(|start| start.elapsed() > self.duration)
//             .unwrap_or(true)
//     }
// }

// #[derive(Clone, Debug)]
// pub struct Clock {
//     frame_delta: Duration,
//     start: Instant,
//     last: Instant,
// }

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

// fn raycast_test(world: &World) {
//     let mouse = world.get::<Mouse>().unwrap();
//     let window = world.get::<Window>().unwrap();

//     if mouse.is_down(MouseButton::Left) {
//         let camera = world.get::<Camera>().unwrap();
//         let world_pos = camera.ndc_to_world(window.screen_to_ndc(mouse.position));
//         let ray = Ray::from_points(camera.eye(), world_pos);

//         let colliders = world.query::<&Collider>();
//         colliders.iter().for_each(|collider| {
//             println!("{:?}", collider.intersects(ray, world));
//         })
//     }

//     pub fn with_transform(mut self, transform: Transform) -> Self {
//         self.transform = transform;
//         self
//     }
// }

pub type World = tecs::World<Event>;

fn main() -> Result<()> {
    pretty_env_logger::init();

    let window = Window::new();

    let renderer = Renderer::new(&window)?;
    let camera = Camera::new(&window);

    let world = World::new()
        .register::<Player>()
        .register::<CopperOre>()
        .register::<TargetDummy>()
        .register::<DebugSphere>()
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

    // let buffer = std::fs::read("assets/scenes/test.scene").unwrap();
    // Scene::load(&world, &mut serde_json::Deserializer::from_slice(&buffer)).unwrap();

    world.spawn(TargetDummy {
        transform: Transform::new(
            Vec3 {
                x: 5.,
                y: 0.,
                z: 0.,
            },
            glam::Quat::default(),
            Vec3 {
                x: 1.,
                y: 1.,
                z: 1.,
            },
        ),
        render: RenderObject {
            mesh: MeshId(String::from("assets/meshes/copper_ore.glb")),
            material: Material::debug_material(),
        },
        defensive_stats: combat::CombatDefensive {
            health: 200,
            fire_resistance: 0,
            earth_resistance: 0,
            lightning_resistance: 0,
            air_resistance: 0,
            nature_resistance: 0,
            is_dead: false,
        },
        collider: Collider {
            // kind: ColliderKind::Aabb(Vec3 { x: 1., y: 1., z: 1. }),
            kind: ColliderKind::Sphere(10.),
            position: Vec3 {
                x: 5.0,
                y: 0.0,
                z: 0.0,
            },
        },
    });

    world.spawn(Player {
        render: RenderObject {
            mesh: MeshId(String::from("assets/meshes/cube.glb")),
            material: Material { colour: Vec4::ONE },
        },
        transform,
        health: player::Health(100.0),
        offensive_stats: combat::CombatOffensive {
            fire: combat::AttackType {
                damage: 0,
                penetration: 0,
            },
            earth: combat::AttackType {
                damage: 0,
                penetration: 0,
            },
            lightning: combat::AttackType {
                damage: 0,
                penetration: 0,
            },
            air: combat::AttackType {
                damage: 0,
                penetration: 0,
            },
            nature: combat::AttackType {
                damage: 0,
                penetration: 0,
            },
            true_damage: 32,
        },
        targeted_entity: player::TargetedEntity::None,
    });

    loop {
        if let State::Stopped = *world.get::<State>().unwrap() {
            break;
        }
        world.tick();
    }

    Ok(())
}
