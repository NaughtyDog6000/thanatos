use glam::Vec4;
use styx::{
    components::{Clicked, Container, HGroup, Text},
    Signal,
};
use tecs::SystemMut;

use crate::{
    event::Event,
    item::{Inventory, Item},
    renderer::{Anchor, Ui},
    window::Keyboard,
    World,
};

#[derive(Default)]
pub struct Equipped {
    weapon: Option<Item>,
}

impl Equipped {
    pub fn equipment(&self) -> impl Iterator<Item = &Item> {
        [self.weapon.as_ref()].into_iter().filter_map(|x| x)
    }
}

#[derive(Default)]
pub struct EquipmentUi {
    open: bool,
    signals: Vec<(Item, Signal)>,
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

        self.signals.drain(..).for_each(|(item, signal)| {
            if ui.signals.get(signal) {
                equipped.weapon = match equipped.weapon {
                    Some(_) => None,
                    None => Some(item)
                }
            }
        });

        let inventory = world.get::<Inventory>().unwrap();

        let equipable = inventory
            .items()
            .map(|(item, _)| item)
            .filter(|item| item.equipable());
        let equipment = equipped.equipment().collect::<Vec<_>>();

        let view = equipable.fold(
            HGroup::new(styx::components::HAlign::Left, 16.0),
            |view, item| {
                let colour = if equipment.contains(&&item) {
                    Vec4::ONE
                } else {
                    Vec4::new(0.5, 0.5, 0.5, 1.0)
                };

                let signal = ui.signals.signal();
                self.signals.push((item, signal));

                view.add(Clicked {
                    signal,
                    child: Text {
                        text: format!("{item}"),
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

pub fn add(world: World) -> World {
    world.with_resource(Equipped::default()).with_system_mut(EquipmentUi::default())
}
