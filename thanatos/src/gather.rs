use glam::{Vec2, Vec3, Vec4};
use nyx::{
    item::{Inventory, ItemStack, LootTable},
    protocol::Serverbound,
};
use rand::Rng;
use styx::components::{self, Container, Offset, Text, VAlign, VGroup};
use tecs::{utils::Name, EntityId, Is};

use crate::{
    collider::Collider,
    net::Connection,
    player::Player,
    renderer::{Anchor, Renderer, Ui},
    transform::Transform,
    window::Keyboard,
    Timer, World,
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
                            colour: Vec4::ONE,
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
                            colour: Vec4::ONE,
                        },
                    }),
            },
        )
    }

    if keyboard.is_down("f") {
        let mut gatherable = world.get_component_mut::<Gatherable>(entity).unwrap();
        let mut conn = world.get_mut::<Connection>().unwrap();
        conn.write(Serverbound::Gather(gatherable.gather()))
            .unwrap();
    }
}
