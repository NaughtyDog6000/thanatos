use crate::{
    camera::Camera, renderer::RenderObject, transform::Transform,
    window::Keyboard, World,
};
use glam::{Quat, Vec3};
use tecs::{impl_archetype, EntityId, Is};
use thanatos_macros::Archetype;

#[derive(Archetype)]
pub struct Player {
    pub render: RenderObject,
    pub transform: Transform,
}

impl Player {
    pub fn tick(world: &World) {
        let keyboard = world.get::<Keyboard>().unwrap();
        let mut camera = world.get_mut::<Camera>().unwrap();

        let (players, _) = world.query::<(EntityId, Is<Player>)>();
        let player = *players.first().unwrap();
        let mut transform = world.get_component_mut::<Transform>(player).unwrap();

        let rotation = Quat::from_rotation_y(camera.theta);

        if keyboard.is_down("w") {
            transform.translation += rotation * Vec3::Z;
        }

        if keyboard.is_down("s") {
            transform.translation -= rotation * Vec3::Z;
        }

        if keyboard.is_down("a") {
            transform.translation += rotation * Vec3::X;
        }

        if keyboard.is_down("d") {
            transform.translation -= rotation * Vec3::X;
        }

        camera.target = transform.translation;
    }
}
