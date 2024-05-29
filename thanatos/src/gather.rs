use glam::Vec3;
use nyx::protocol::Serverbound;
use serde::{Deserialize, Serialize};
use tecs::{EntityId, Is};

use crate::{
    collider::Collider, interact::Interactable, net::Connection, player::Player, renderer::Ui,
    transform::Transform, Timer, World,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
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
        let (gatherables, mut interactables, entities) =
            world.query::<(&Gatherable, &mut Interactable, EntityId)>();

        interactables.for_each(|interactable| interactable.priority = f32::MAX);

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
    if interactable.signal.map(|signal| ui.signals.get(signal)).unwrap_or_default() {
        let mut gatherable = world.get_component_mut::<Gatherable>(entity).unwrap();
        let mut conn = world.get_mut::<Connection>().unwrap();
        conn.write(Serverbound::Gather(gatherable.gather()))
            .unwrap();
    }
}
