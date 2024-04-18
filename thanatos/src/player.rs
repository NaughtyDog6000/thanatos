use crate::{
    camera::Camera, renderer::RenderObject, transform::Transform, window::Keyboard, Clock, World,
};
use glam::{Quat, Vec3};
use tecs::{impl_archetype, Is};
use thanatos_macros::Archetype;

const SPEED: f32 = 5.0;

#[derive(Archetype)]
pub struct Player {
    pub render: RenderObject,
    pub transform: Transform,
}

impl Player {
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
