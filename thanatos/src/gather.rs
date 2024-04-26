use glam::Vec3;
use rand::Rng;
use tecs::{EntityId, Is};

use crate::{
    collider::Collider,
    item::{Inventory, ItemStack},
    player::Player,
    transform::Transform,
    window::Keyboard,
    Timer, World,
};

#[derive(Default)]
pub struct LootTable {
    entries: Vec<(f32, Vec<ItemStack>)>,
}

impl LootTable {
    pub fn add(mut self, probability: f32, loot: Vec<ItemStack>) -> Self {
        self.entries.push((probability, loot));
        self
    }

    pub fn pick(&self) -> &[ItemStack] {
        let mut rng = rand::thread_rng();
        let mut p: f32 = rng.gen();
        self.entries
            .iter()
            .find_map(|(probability, items)| {
                p -= probability;
                if p < 0.0 {
                    Some(items)
                } else {
                    None
                }
            })
            .unwrap()
    }
}

pub struct Gatherable {
    pub collider: Collider,
    pub loot: LootTable,
    pub timer: Timer,
}

impl Gatherable {
    pub fn gatherable(&self, position: Vec3, world: &World) -> bool {
        self.collider.within(position, world)
    }

    pub fn gather(&mut self) -> &[ItemStack] {
        self.timer.start();
        self.loot.pick()
    }
}

pub fn tick(world: &World) {
    let keyboard = world.get::<Keyboard>().unwrap();
    if keyboard.is_down("f") {
        let entity = {
            let (gatherables, entities) = world.query::<(&Gatherable, EntityId)>();
            let (transforms, _) = world.query::<(&Transform, Is<Player>)>();
            let transform = transforms.iter().next().unwrap();
            let Some((_, entity)) = gatherables
                .iter()
                .zip(entities.into_iter())
                .filter(|(gatherable, _)| gatherable.timer.done())
                .filter(|(gatherable, _)| gatherable.gatherable(transform.translation, world))
                .next()
            else {
                return;
            };
            entity
        };

        let mut inventory = world.get_mut::<Inventory>().unwrap();
        let mut gatherable = world.get_component_mut::<Gatherable>(entity).unwrap();
        gatherable
            .gather()
            .iter()
            .copied()
            .for_each(|stack| inventory.add(stack));
        println!("{inventory:?}");
    }
}
