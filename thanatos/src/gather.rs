use glam::{Vec2, Vec3, Vec4};
use nyx::{
    item::{Inventory, ItemStack, LootTable},
    protocol::Serverbound,
};
use rand::Rng;
use styx::components::{self, Container, Offset, Text, VAlign, VGroup};
use tecs::{utils::Name, EntityId, Is};

use crate::{
    collider::Collider, interact::Interactable, net::Connection, player::Player, renderer::{Anchor, Renderer, Ui}, transform::Transform, window::Keyboard, Timer, World
};

pub struct Gatherable {
    pub collider: Collider,
    pub loot: usize,
    pub timer: Timer,
}

impl Gatherable {
    pub fn gatherable(&self, position: Vec3) -> bool {
        self.collider.within(position)
    }

    pub fn gather(&mut self) -> usize {
        self.timer.start();
        self.loot
    }
}

pub fn tick(world: &World) {
    let entity = {
        let (gatherables, entities) = world.query::<(&Gatherable, EntityId)>();
        let (transforms, _) = world.query::<(&Transform, Is<Player>)>();
        let transform = transforms.iter().next().unwrap();
        let Some((_, entity)) = gatherables
            .iter()
            .zip(entities)
            .filter(|(gatherable, _)| gatherable.timer.done())
            .filter(|(gatherable, _)| gatherable.gatherable(transform.translation))
            .next()
        else {
            return;
        };
        entity
    };

    
    let mut interactable = world.get_component_mut::<Interactable>(entity).unwrap();
    interactable.priority = 0.0;

    let ui = world.get::<Ui>().unwrap();
    if ui.signals.get(interactable.signal) {
        let mut gatherable = world.get_component_mut::<Gatherable>(entity).unwrap();
        let mut conn = world.get_mut::<Connection>().unwrap();
        conn.write(Serverbound::Gather(gatherable.gather()))
            .unwrap();
    }

}
