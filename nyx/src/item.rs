use std::{collections::HashMap, fmt::Display};

use rand::Rng;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Item {
    Wood,
    CopperOre,
    CopperIngot,
    CopperSword,
}

impl Item {
    pub fn equipable(&self) -> bool {
        match self {
            Self::CopperSword => true,
            _ => false,
        }
    }
}

impl Display for Item {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Wood => "Wood",
                Self::CopperOre => "Copper Ore",
                Self::CopperIngot => "Copper Ingot",
                Self::CopperSword => "Copper Sword",
            }
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ItemStack {
    pub item: Item,
    pub quantity: usize,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Recipe {
    pub inputs: Vec<ItemStack>,
    pub outputs: Vec<ItemStack>,
}

impl Recipe {
    pub fn craftable(&self, inventory: &[ItemStack]) -> bool {
        self.inputs.iter().all(|stack| {
            stack.quantity
                <= inventory
                    .iter()
                    .find_map(|s| {
                        if s.item == stack.item {
                            Some(s.quantity)
                        } else {
                            None
                        }
                    })
                    .unwrap_or_default()
        })
    }
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

    pub fn remove(&mut self, stack: ItemStack) -> Option<()> {
        self.0
            .get_mut(&stack.item)
            .map(|quantity| *quantity -= stack.quantity)
    }

    pub fn get(&self, item: Item) -> Option<usize> {
        self.0.get(&item).copied()
    }

    pub fn set(&mut self, stack: ItemStack) {
        match self.0.get_mut(&stack.item) {
            Some(quantity) => *quantity = stack.quantity,
            None => {
                self.0.insert(stack.item, stack.quantity);
            }
        }
    }

    pub fn items(&self) -> impl Iterator<Item = ItemStack> {
        self.0
            .clone()
            .into_iter()
            .map(|(item, quantity)| ItemStack { item, quantity })
    }
}

#[derive(Default, Clone)]
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

