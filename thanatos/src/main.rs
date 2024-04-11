mod assets;
mod camera;
mod collider;
mod event;
mod graphics;
mod transform;
mod window;

use std::time::{Duration, Instant};

use crate::{camera::Camera, collider::Ray, window::Window};
use anyhow::Result;
use assets::{Material, Mesh};
use collider::Collider;
use event::Event;
use glam::{Vec2, Vec3, Vec4};
use graphics::{RenderObject, Renderer};
use tecs::impl_archetype;
use thanatos_macros::Archetype;
use transform::Transform;
use window::{Keyboard, Mouse};
use winit::event::MouseButton;

#[derive(Archetype)]
struct CopperOre {
    render: RenderObject,
    transform: Transform,
    collider: Collider,
}

#[derive(Archetype)]
struct Tree {
    render: RenderObject,
}

#[derive(Clone, Debug)]
pub struct Clock {
    frame_delta: Duration,
    start: Instant,
    last: Instant,
}

impl Clock {
    pub fn add(world: World) -> World {
        world
            .with_resource(Self {
                frame_delta: Duration::ZERO,
                start: Instant::now(),
                last: Instant::now(),
            })
            .with_ticker(Self::tick)
    }

    pub fn tick(world: &mut World) {
        let mut clock = world.get_mut::<Clock>().unwrap();
        let now = Instant::now();
        clock.frame_delta = now - clock.last;
        clock.last = now;
    }
}

#[derive(Copy, Clone, Debug)]
pub enum State {
    Stopped,
    Running,
}

fn raycast_test(world: &mut World) {
    let mouse = world.get::<Mouse>().unwrap();
    let window = world.get::<Window>().unwrap();

    if mouse.is_down(MouseButton::Left) {
        let camera = world.get::<Camera>().unwrap();
        let world_pos = camera.ndc_to_world(window.screen_to_ndc(mouse.position));
        let ray = Ray::from_points(camera.eye, world_pos);

        let colliders = world.query::<&Collider>();
        colliders.iter().for_each(|collider| {
            println!("{:?}", collider.intersects(ray));
        })
    }
}

pub type World = tecs::World<Event>;

#[tokio::main]
async fn main() -> Result<()> {
    pretty_env_logger::init();

    let window = Window::new();

    let renderer = Renderer::new(&window)?;
    let camera = Camera::new(&window);

    let mut assets = assets::Manager::new();
    let copper_ore = assets.add_mesh(Mesh::load("assets/meshes/cube.glb", &renderer)?);
    let tree = assets.add_mesh(Mesh::load("assets/meshes/tree.glb", &renderer)?);
    let material = assets.add_material(Material { colour: Vec4::X + Vec4::Z + Vec4::W });
    let mut world = World::new()
        .with_resource(State::Running)
        .with_resource(assets)
        .with(window.add())
        .with(renderer.add())
        .with(camera.add())
        .with(Clock::add)
        .with_ticker(raycast_test)
        .with_ticker(|world| {
            let clock = world.get::<Clock>().unwrap();
            println!("FPS: {}", 1.0 / clock.frame_delta.as_secs_f32());
        })
        .with_handler(|world, event| match event {
            Event::Stop => {
                *world.get_mut::<State>().unwrap() = State::Stopped;
            }
            _ => (),
        });

    let mut transform = Transform::IDENTITY;
    transform.translation += Vec3::ZERO;

    world.spawn(CopperOre {
        render: RenderObject { mesh: copper_ore, material },
        transform,
        collider: Collider {
            kind: collider::ColliderKind::Aabb(Vec3::ONE),
            position: Vec3::ZERO,
        },
    });
    world.spawn(Tree {
        render: RenderObject { mesh: tree, material },
    });

    loop {
        if let State::Stopped = *world.get::<State>().unwrap() {
            break;
        }
        world.tick();
    }

    let renderer = world.remove::<Renderer>().unwrap();
    unsafe { renderer.ctx.device.device_wait_idle(); }
    let manager = world.remove::<assets::Manager>().unwrap();
    manager.destroy(&renderer);
    renderer.destroy();

    Ok(())
}
