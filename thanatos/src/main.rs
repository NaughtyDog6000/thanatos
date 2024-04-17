mod assets;
mod camera;
mod collider;
mod event;
mod gather;
mod item;
mod net;
mod player;
mod renderer;
mod transform;
mod window;

use std::time::{Duration, Instant};

use crate::{camera::Camera, collider::Ray, window::Window};
use anyhow::Result;
use assets::{Material, Mesh};
use collider::{Collider, ColliderKind};
use event::Event;
use gather::{Gatherable, LootTable};
use glam::{Vec2, Vec3, Vec4};
use item::{Inventory, Item, ItemStack};
use net::{Connection, OtherPlayer};
use player::Player;
use renderer::{RenderObject, Renderer};
use tecs::impl_archetype;
use thanatos_macros::Archetype;
use transform::Transform;
use window::{Keyboard, Mouse};
use winit::event::MouseButton;

#[derive(Archetype)]
struct CopperOre {
    pub render: RenderObject,
    pub transform: Transform,
    pub gatherable: Gatherable,
}

#[derive(Archetype)]
struct Tree {
    pub render: RenderObject,
}

struct Timer {
    start: Option<Instant>,
    pub duration: Duration,
}

impl Timer {
    pub fn new(duration: Duration) -> Self {
        Self {
            start: None,
            duration,
        }
    }

    pub fn start(&mut self) {
        self.start = Some(Instant::now())
    }

    pub fn done(&self) -> bool {
        self.start
            .map(|start| start.elapsed() > self.duration)
            .unwrap_or(true)
    }
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

    pub fn tick(world: &World) {
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

fn raycast_test(world: &World) {
    let mouse = world.get::<Mouse>().unwrap();
    let window = world.get::<Window>().unwrap();

    if mouse.is_down(MouseButton::Left) {
        let camera = world.get::<Camera>().unwrap();
        let world_pos = camera.ndc_to_world(window.screen_to_ndc(mouse.position));
        let ray = Ray::from_points(camera.eye(), world_pos);

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
    let cube = assets.add_mesh(Mesh::load("assets/meshes/cube.glb", &renderer)?);
    let copper_ore = assets.add_mesh(Mesh::load("assets/meshes/copper_ore.glb", &renderer)?);
    let white = assets.add_material(Material { colour: Vec4::ONE });
    let orange = assets.add_material(Material {
        colour: Vec4::new(1.0, 0.5, 0.0, 1.0),
    });
    let mut world = World::new()
        .register::<Player>()
        .register::<CopperOre>()
        .with_resource(State::Running)
        .with(Connection::add)
        .with_resource(assets)
        .with_resource(Inventory::default())
        .with(window.add())
        .with(renderer.add())
        .with(camera.add())
        .with(Clock::add)
        .with_ticker(raycast_test)
        .with_handler(|world, event| match event {
            Event::Stop => {
                *world.get_mut::<State>().unwrap() = State::Stopped;
            }
            _ => (),
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
    });
    world.spawn(CopperOre {
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
            loot: LootTable::default().add(
                1.0,
                vec![ItemStack {
                    item: Item::CopperOre,
                    quantity: 2,
                }],
            ),
            timer: Timer::new(Duration::from_secs(5)),
        },
    });

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
