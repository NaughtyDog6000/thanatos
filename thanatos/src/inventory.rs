use glam::Vec4;
use nyx::{
    item::{Inventory, Item, ItemStack},
    protocol::Clientbound,
};
use styx::components::{Container, HAlign, HGroup, Text};
use tecs::SystemMut;

use crate::{
    colours::rarity_colour, event::Event, renderer::{Anchor, Ui}, window::Keyboard, World
};

pub struct InventoryUi {
    open: bool,
}

impl InventoryUi {
    pub fn new() -> Self {
        Self { open: false }
    }
}

impl SystemMut<Event> for InventoryUi {
    fn tick(&mut self, world: &World) {
        let keyboard = world.get::<Keyboard>().unwrap();
        if keyboard.pressed("i") {
            self.open = !self.open;
        }

        if !self.open {
            return;
        }
        let mut ui = world.get_mut::<Ui>().unwrap();
        let inventory = world.get::<Inventory>().unwrap();

        let stacks = inventory.items().fold(
            HGroup::new(HAlign::Left, 4.0),
            |stacks,
             ItemStack {
                 item: Item { kind, rarity },
                 quantity,
             }| {
                stacks.add(Text {
                    text: format!("{kind} x {quantity}"),
                    font_size: 24.0,
                    font: ui.font.clone(),
                    colour: rarity_colour(rarity),
                })
            },
        );
        let container = Container {
            padding: 16.0,
            radius: 8.0,
            colour: Vec4::new(0.1, 0.1, 0.1, 1.0),
            child: stacks,
        };
        let padded = Container {
            padding: 16.0,
            radius: 0.0,
            colour: Vec4::ZERO,
            child: container,
        };

        ui.add(Anchor::BottomRight, padded);
    }
}

fn handle_net(world: &World, event: &Event) {
    if let Event::Recieved(Clientbound::SetStack(stack)) = event {
        let mut inventory = world.get_mut::<Inventory>().unwrap();
        inventory.set(*stack);
    }
}

pub fn add(world: World) -> World {
    world
        .with_resource(Inventory::default())
        .with_handler(handle_net)
        .with_system_mut(InventoryUi::new())
}
