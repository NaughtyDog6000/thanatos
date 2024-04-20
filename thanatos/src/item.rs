use std::{cell::Cell, collections::HashMap, fmt::Display};

use glam::Vec4;
use styx::components::{Container, HAlign, HGroup, Text};
use tecs::System;

use crate::{event::Event, renderer::{Anchor, Ui}, window::Keyboard, World};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Item {
    Wood,
    CopperOre,
    CopperIngot,
    CopperSword,
}

impl Display for Item {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            Self::Wood => "Wood",
            Self::CopperOre => "Copper Ore",
            Self::CopperIngot => "Copper Ingot",
            Self::CopperSword => "Copper Sword"
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ItemStack {
    pub item: Item,
    pub quantity: usize,
}

#[derive(Default, Debug)]
pub struct Inventory(HashMap<Item, usize>);

impl Inventory {
    pub fn add(&mut self, stack: ItemStack) {
        match self.0.get_mut(&stack.item) {
            Some(quantity) => *quantity += stack.quantity,
            None => {
                self.0.insert(stack.item, stack.quantity);
            }
        }
    }

    pub fn get(&self, item: Item) -> Option<usize> {
        self.0.get(&item).copied()
    }

    pub fn items(&self) -> impl Iterator<Item = (Item, usize)> + '_ {
        self.0.iter().map(|(item, quantity)| (*item, *quantity))
    }
}

pub struct InventoryUi {
    open: Cell<bool>,
}

impl InventoryUi {
    pub fn new() -> Self {
        Self { open: Cell::new(false) }
    }

    pub fn add(world: World) -> World {
        world.with_system(Self::new())
    }
}

impl System<Event> for InventoryUi {
    fn tick(&self, world: &World) {
        let keyboard = world.get::<Keyboard>().unwrap();
        if keyboard.is_down("i") { self.open.set(true); }

        if !self.open.get() {
            return;
        }
        let mut ui = world.get_mut::<Ui>().unwrap();
        let inventory = world.get::<Inventory>().unwrap();

        let stacks = inventory.items().fold(
            HGroup::new(HAlign::Left, 4.0),
            |stacks, (item, quantity)| {
                stacks.add(Text {
                    text: format!("{item} x {quantity}"),
                    font_size: 24.0,
                    font: ui.font.clone(),
                })
            },
        );
        let container = Container {
            padding: 16.0,
            radius: 8.0,
            colour: Vec4::new(0.1, 0.1, 0.1, 1.0),
            child: stacks
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
