use glam::Vec4;
use nyx::{equipment::{EquipmentId, EquipmentInventory, Equipped}, item::{Inventory, Item, ItemStack}, protocol::Clientbound};
use styx::{
    components::{Clicked, Container, HGroup, Text},
    Signal,
};
use tecs::SystemMut;

use crate::{
    colours::rarity_colour, event::Event, renderer::{Anchor, Ui}, window::Keyboard, World
};

#[derive(Default)]
pub struct EquipmentUi {
    open: bool,
    signals: Vec<(EquipmentId, Signal)>,
}

impl SystemMut<Event> for EquipmentUi {
    fn tick(&mut self, world: &World) {
        let keyboard = world.get::<Keyboard>().unwrap();
        if keyboard.pressed("e") {
            self.open = !self.open;
        }

        if !self.open {
            return;
        }

        let mut ui = world.get_mut::<Ui>().unwrap();
        let mut equipped = world.get_mut::<Equipped>().unwrap();

        self.signals.drain(..).for_each(|(equipment, signal)| {
            if ui.signals.get(signal) {
                equipped.weapon = match equipped.weapon {
                    Some(current) if current == equipment => None,
                    _ => Some(equipment),
                }
            }
        });

        let inventory = world.get::<EquipmentInventory>().unwrap();
        let equipment = equipped.equipment().collect::<Vec<_>>();

        let view = inventory.0.iter().fold(
            HGroup::new(styx::components::HAlign::Left, 16.0),
            |view, equipable| {
                let mut colour = rarity_colour(equipable.rarity);
                if !equipment.iter().any(|id| equipable.id == *id) {
                    colour *= Vec4::new(0.5, 0.5, 0.5, 1.0)
                }

                let signal = ui.signals.signal();
                self.signals.push((equipable.id, signal));

                view.add(Clicked {
                    signal,
                    child: Text {
                        text: format!("{}", equipable.kind),
                        font_size: 48.0,
                        font: ui.font.clone(),
                        colour,
                    },
                })
            },
        );

        let view = Container {
            padding: 32.0,
            colour: Vec4::new(0.1, 0.1, 0.1, 1.0),
            radius: 8.0,
            child: view,
        };
        ui.add(Anchor::Center, view);
    }
}

fn net(world: &World, event: &Event) { 
    match event {
        Event::Recieved(Clientbound::AddEquipment(piece)) => {
            let mut equipment = world.get_mut::<EquipmentInventory>().unwrap();
            equipment.0.push(piece.clone());
        }
        _ => ()
    }
}

pub fn add(world: World) -> World {
    world
        .with_resource(Equipped::default())
        .with_resource(EquipmentInventory(Vec::new()))
        .with_system_mut(EquipmentUi::default())
        .with_handler(net)
}
