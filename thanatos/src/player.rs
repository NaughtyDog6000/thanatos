use crate::{
    camera::Camera, renderer::RenderObject, transform::Transform, window::Keyboard, Clock, World,
};
use glam::{Quat, Vec3};
use serde::{Deserialize, Serialize};
use tecs::prelude::*;

const SPEED: f32 = 5.0;

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Health(pub f32);

impl Default for Health {
    fn default() -> Self {
        Self(100.0)
    }
}

#[derive(Archetype, Clone, Serialize, Deserialize)]
pub struct Player {
    pub render: RenderObject,
    pub transform: Transform,
    #[serde(skip)]
    pub health: Health,
}

impl Player {
    pub fn death(world: &World) {
        let (mut health, mut transform, _) =
            world.query_one::<(&mut Health, &mut Transform, Is<Player>)>();

        if health.0 < 0.0 {
            transform.translation = Vec3::ZERO;
            health.0 = 100.0;
        }
    }

    pub fn tick(world: &World) {
        let keyboard = world.get::<Keyboard>().unwrap();
        let mut camera = world.get_mut::<Camera>().unwrap();
        let clock = world.get::<Clock>().unwrap();

        let (mut transform, _) = world.query_one::<(&mut Transform, Is<Player>)>();

        let rotation = Quat::from_rotation_y(camera.theta);

        if keyboard.is_down("w") {
            transform.translation += rotation * Vec3::Z * SPEED * clock.delta.as_secs_f32();
        }

        if keyboard.is_down("s") {
            transform.translation -= rotation * Vec3::Z * SPEED * clock.delta.as_secs_f32();
        }

        if keyboard.is_down("a") {
            transform.translation += rotation * Vec3::X * SPEED * clock.delta.as_secs_f32();
        }

        if keyboard.is_down("d") {
            transform.translation -= rotation * Vec3::X * SPEED * clock.delta.as_secs_f32();
        }

        camera.target = transform.translation;
    }
}

pub fn add(world: World) -> World {
    world.with_ticker(Player::tick).with_ticker(Player::death)
}
