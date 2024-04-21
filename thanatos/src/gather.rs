use glam::{Vec2, Vec3, Vec4};
use rand::Rng;
use styx::components::{self, Container, Offset, Text, VAlign, VGroup};
use tecs::{utils::Name, EntityId, Is};

use crate::{
    collider::Collider,
    item::{Inventory, ItemStack},
    player::Player,
    renderer::{Anchor, Renderer, Ui},
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
    pub fn gatherable(&self, position: Vec3) -> bool {
        self.collider.within(position)
    }

    pub fn gather(&mut self) -> &[ItemStack] {
        self.timer.start();
        self.loot.pick()
    }
}

pub fn tick(world: &World) {
    let keyboard = world.get::<Keyboard>().unwrap();
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

    {
        let mut ui = world.get_mut::<Ui>().unwrap();
        let font = ui.font.clone();
        let name = world.get_component::<Name>(entity);
        ui.add(
            Anchor::Cursor,
            Offset {
                offset: Vec2::new(32.0, 32.0),
                child: VGroup::new(VAlign::Center, 4.0)
                    .add(Container {
                        padding: 4.0,
                        colour: Vec4::new(0.2, 0.2, 0.2, 1.0),
                        radius: 4.0,
                        child: Text {
                            font: font.clone(),
                            text: String::from("F"),
                            font_size: 24.0,
                        },
                    })
                    .add(Container {
                        padding: 4.0,
                        colour: Vec4::new(0.1, 0.1, 0.1, 1.0),
                        radius: 4.0,
                        child: Text {
                            font: font.clone(),
                            text: match name {
                                Some(name) => format!("Gather {name}"),
                                None => String::from("Gather"),
                            },
                            font_size: 16.0,
                        },
                    }),
            },
        )
    }

    if keyboard.is_down("f") {
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
