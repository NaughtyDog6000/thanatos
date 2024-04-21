use crate::{
    camera::Camera, combat::CombatOffensive, renderer::RenderObject, transform::Transform, window::Keyboard, Clock, World, combat::Attack
};
use glam::{Quat, Vec3};
use tecs::{impl_archetype, EntityId, Is};
use thanatos_macros::Archetype;
const SPEED: f32 = 5.0;

pub enum TargetedEntity {
    None,
    EntityId(EntityId),
    Position(Vec3)
}

#[derive(Archetype)]
pub struct Player {
    pub render: RenderObject,
    pub transform: Transform,
    pub offensive_stats: CombatOffensive,
    pub targeted_entity: TargetedEntity,
}

impl Player {
    pub fn tick(world: &World) {
        let keyboard = world.get::<Keyboard>().unwrap();
        let mut camera = world.get_mut::<Camera>().unwrap();
        let clock = world.get::<Clock>().unwrap();

        let (mut transform, _) = world.query_one::<(&mut Transform, Is<Player>)>();

        let rotation = Quat::from_rotation_y(camera.theta);

        if keyboard.is_down("w") {
            transform.translation += rotation * Vec3::Z * SPEED * clock.frame_delta.as_secs_f32();
        }

        if keyboard.is_down("s") {
            transform.translation -= rotation * Vec3::Z * SPEED * clock.frame_delta.as_secs_f32();
        }

        if keyboard.is_down("a") {
            transform.translation += rotation * Vec3::X * SPEED * clock.frame_delta.as_secs_f32();
        }

        if keyboard.is_down("d") {
            transform.translation -= rotation * Vec3::X * SPEED * clock.frame_delta.as_secs_f32();
        }



        camera.target = transform.translation;
    }
}
