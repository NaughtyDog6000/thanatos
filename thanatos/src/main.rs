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
use assets::{Material, Mesh};
use collider::{Collider, ColliderKind};
use event::Event;
use gather::Gatherable;
use glam::{Quat, Vec3, Vec4};
use interact::Interactable;
use net::Connection;
use nyx::{
    data,
    item::{Item, ItemStack},
    task::Proficiencies,
};
use player::{Health, Player};
use renderer::{RenderObject, Renderer};
use std::time::Duration;
use tecs::{
    impl_archetype,
    utils::{Clock, Name, State, Timer},
};
use thanatos_macros::Archetype;
use transform::Transform;

#[derive(Archetype)]
struct CopperOre {
    pub render: RenderObject,
    pub transform: Transform,
    pub gatherable: Gatherable,
    pub interactable: Interactable,
    pub name: Name,
}

impl CopperOre {
    pub fn new(world: &World) -> Result<Self> {
        let mut assets = world.get_mut::<assets::Manager>().unwrap();

        let copper_ore = assets.add_mesh(Mesh::load("assets/meshes/copper_ore.glb")?);
        let orange = assets.add_material(Material {
            colour: Vec4::new(1.0, 0.5, 0.0, 1.0),
        });

        Ok(CopperOre {
            render: RenderObject {
                mesh: copper_ore,
                material: orange,
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

    let mut assets = assets::Manager::new();
    let cube = assets.add_mesh(Mesh::load("assets/meshes/cube.glb")?);
    let white = assets.add_material(Material { colour: Vec4::ONE });

    let mut world = World::new()
        .register::<Player>()
        .register::<CopperOre>()
        .with_resource(State::Running)
        .with_resource(Proficiencies::default())
        .with(Connection::add)
        .with_resource(assets)
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
        .with(net::add(cube, white));

    let mut transform = Transform::IDENTITY;
    transform.translation += Vec3::ZERO;

    world.spawn(Player {
        render: RenderObject {
            mesh: cube,
            material: white,
        },
        transform,
        health: Health(100.0),
    });

    world.spawn(CopperOre::new(&world)?.with_transform(Transform {
        translation: Vec3::ONE,
        rotation: Quat::IDENTITY,
        scale: Vec3::new(3.0, 1.0, 2.0),
    }));

    loop {
        if let State::Stopped = *world.get::<State>().unwrap() {
            break;
        }
        world.tick();
    }

    // Remove early to drop GPU resources
    {
        world.remove::<assets::Manager>().unwrap();
    }

    Ok(())
}
