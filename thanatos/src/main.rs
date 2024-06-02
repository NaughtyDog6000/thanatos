mod assets;
mod camera;
mod casting;
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
mod targeting;
mod transform;
mod uiutils;
mod window;

use crate::{camera::Camera, window::Window};
use anyhow::Result;
use assets::{Material, MeshCache, MeshId};
use casting::Skill;
use collider::{Collider, ColliderKind};
use event::Event;
use gather::Gatherable;
use glam::{Quat, Vec3, Vec4};
use interact::Interactable;
use net::Connection;
use nyx::task::Proficiencies;
use player::Player;
use renderer::{RenderObject, Renderer};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use targeting::{Selectable, SelectedEntity};
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
    pub selectable: Selectable,
}

#[derive(Archetype, Clone, Serialize, Deserialize)]
struct DebugSphere {
    pub transform: Transform,
    pub render: RenderObject,
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
    println!(
        "Logging Level: {}",
        std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string())
    );
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
        .with(targeting::add)
        .with(casting::add)
        .with_handler(|world, event| match event {
            Event::Stop => {
                *world.get_mut::<State>().unwrap() = State::Stopped;
            }
            _ => (),
        })
        .with_ticker(|world| {
            let clock = world.get::<Clock>().unwrap();
            // println!("FPS: {}", 1.0 / clock.delta.as_secs_f32());
        })
        .with_ticker(Player::tick)
        .with_ticker(gather::tick)
        .with_ticker(combat::tick)
        .with_ticker(targeting::tick)
        .with(net::add);

    let mut transform = Transform::IDENTITY;
    transform.translation += Vec3::ZERO;

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
            mesh: MeshId(String::from(
                "assets/meshes/mannequin_armor_dummy_medieval_game_prop_Resized.glb",
            )),
            material: Material::DEBUG_MATERIAL,
        },
        defensive_stats: combat::CombatDefensive {
            health: 100,
            max_health: 100,
            fire_resistance: 0,
            earth_resistance: 0,
            lightning_resistance: 0,
            air_resistance: 0,
            nature_resistance: 0,
        },
        collider: Collider {
            // kind: ColliderKind::Aabb(Vec3 { x: 1., y: 1., z: 1. }),
            kind: ColliderKind::Sphere(3.),
            position: Vec3 {
                x: 5.0,
                y: 0.0,
                z: 0.0,
            },
        },
        selectable: Selectable {
            selected_material: Material::RED,
            unselected_material: Material::DEBUG_MATERIAL,
            selected_name: "Target1".to_string(),
        },
    });

    world.spawn(TargetDummy {
        transform: Transform::new(
            Vec3 {
                x: -5.,
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
            mesh: MeshId(String::from(
                "assets/meshes/mannequin_armor_dummy_medieval_game_prop_Resized.glb",
            )),
            material: Material::DEBUG_MATERIAL,
        },
        defensive_stats: combat::CombatDefensive {
            health: 200,
            max_health: 200,
            fire_resistance: 0,
            earth_resistance: 0,
            lightning_resistance: 0,
            air_resistance: 0,
            nature_resistance: 0,
        },
        collider: Collider {
            // kind: ColliderKind::Aabb(Vec3 { x: 1., y: 1., z: 1. }),
            kind: ColliderKind::Sphere(3.),
            position: Vec3 {
                x: -5.0,
                y: 0.0,
                z: 0.0,
            },
        },
        selectable: Selectable {
            selected_material: Material::RED,
            unselected_material: Material::DEBUG_MATERIAL,
            selected_name: "Target2".to_string(),
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
            equiped_skills: vec![
                Skill {
                    name: "fireball".to_string(),
                    description: "strike an area with a fireball doing {DAMAGE} damage."
                        .to_string(),
                    cooldown: 12.0,
                    targeting_method: casting::SkillTargeting::Point { range: 10.0 },
                    cast_type: casting::CastType::Charge {
                        charge_duration: 4.0,
                        stationary_cast: true,
                    },
                    effects: [
                        casting::Effect {
                            area_of_effect: Some(2.0),
                            variant: casting::EffectType::Damage(casting::DamageEffect {
                                true_damage: 0,
                                melee_damage: 0,
                                ranged_damage: 0,
                                magic_damage: 20,
                            }),
                        },
                        casting::Effect {
                            area_of_effect: None,
                            variant: casting::EffectType::BufDebuf,
                        },
                    ]
                    .to_vec(),
                },
                Skill {
                    name: "IceDart".to_string(),
                    description: "strike an area with a Shard of ICE doing {DAMAGE} damage."
                        .to_string(),
                    cooldown: 12.0,
                    targeting_method: casting::SkillTargeting::Point { range: 10.0 },
                    cast_type: casting::CastType::Charge {
                        charge_duration: 4.0,
                        stationary_cast: true,
                    },
                    effects: [
                        casting::Effect {
                            area_of_effect: Some(2.0),
                            variant: casting::EffectType::Damage(casting::DamageEffect {
                                true_damage: 0,
                                melee_damage: 0,
                                ranged_damage: 0,
                                magic_damage: 20,
                            }),
                        },
                        casting::Effect {
                            area_of_effect: None,
                            variant: casting::EffectType::BufDebuf,
                        },
                    ]
                    .to_vec(),
                },
            ],
        },
        targeted_entity: SelectedEntity::None,
    });

    world.spawn(CopperOre::new(&world)?.with_transform(Transform {
        translation: Vec3::ONE,
        rotation: Quat::IDENTITY,
        scale: Vec3::new(3.0, 1.0, 2.0),
    }));
    // let mut scene = Scene::default();
    // scene.from_world(&world);

    // let mut buffer: Vec<u8> = Vec::new();
    // let mut serializer = serde_json::Serializer::pretty(&mut buffer);
    // scene.save(&world, &mut serializer).unwrap();
    // std::fs::write("assets/scenes/test.scene", buffer).unwrap();

    // let buffer = std::fs::read("assets/scenes/test.scene").unwrap();
    // Scene::load(&world, &mut serde_json::Deserializer::from_slice(&buffer)).unwrap();

    loop {
        if let State::Stopped = *world.get::<State>().unwrap() {
            break;
        }
        world.tick();
    }

    Ok(())
}
