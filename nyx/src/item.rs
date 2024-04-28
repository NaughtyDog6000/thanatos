use std::{collections::HashMap, fmt::Display};

use rand::Rng;

use crate::equipment::{EquipmentKind, Passive};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Tag {
    Mining,
    Smelting,
    Weaponsmithing,
    Alchemy,
    Copper,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ItemKind {
    CopperOre,
    CopperIngot,
    FireDamageReagent,
}

impl Display for ItemKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::CopperOre => "Copper Ore",
                Self::CopperIngot => "Copper Ingot",
                Self::FireDamageReagent => "Fire Damage Reagent",
            }
        )
    }
}

impl ItemKind {
    pub fn tags(&self) -> Vec<Tag> {
        match self {
            Self::CopperOre => vec![Tag::Mining, Tag::Copper],
            Self::CopperIngot => vec![Tag::Smelting, Tag::Copper],
            Self::FireDamageReagent => vec![Tag::Alchemy],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Rarity {
    Common,
    Uncommon,
    Rare,
    Epic,
    Legendary,
}

pub const RARITIES: [Rarity; 5] = [
    Rarity::Common,
    Rarity::Uncommon,
    Rarity::Rare,
    Rarity::Epic,
    Rarity::Legendary,
];

impl Rarity {
    pub fn next(&self) -> Self {
        match self {
            Self::Common => Self::Uncommon,
            Self::Uncommon => Self::Rare,
            Self::Rare => Self::Epic,
            Self::Epic => Self::Legendary,
            Self::Legendary => Self::Legendary,
        }
    }

    pub fn index(&self) -> usize {
        match self {
            Self::Common => 0,
            Self::Uncommon => 1,
            Self::Rare => 2,
            Self::Epic => 3,
            Self::Legendary => 4,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Item {
    pub kind: ItemKind,
    pub rarity: Rarity,
}

impl Item {
    pub fn passive(&self) -> Option<Passive> {
        match self.kind {
            ItemKind::FireDamageReagent => {
                Some(Passive::FireDamage(0.2 + 0.1 * self.rarity.index() as f32))
            }
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ItemStack {
    pub item: Item,
    pub quantity: usize,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum RecipeOutput {
    Item(ItemKind),
    Equipment(EquipmentKind),
}

impl RecipeOutput {
    pub fn tags(&self) -> Vec<Tag> {
        match self {
            RecipeOutput::Item(item) => item.tags(),
            RecipeOutput::Equipment(equipment) => equipment.tags(),
        }
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Recipe {
    pub inputs: Vec<(ItemKind, usize)>,
    pub output: RecipeOutput,
}

impl Recipe {
    pub fn craftable(&self, inventory: &[ItemStack], rarities: &[Rarity]) -> bool {
        self.inputs
            .iter()
            .zip(rarities)
            .all(|((kind, quantity), rarity)| {
                *quantity
                    <= inventory
                        .iter()
                        .find_map(|s| {
                            if s.item
                                == (Item {
                                    kind: *kind,
                                    rarity: *rarity,
                                })
                            {
                                Some(s.quantity)
                            } else {
                                None
                            }
                        })
                        .unwrap_or_default()
            })
    }

    pub fn rarity_chances(&self, rarities: &[Rarity], rank_up: f32) -> Vec<f32> {
        let rank_up = rank_up.min(1.0);
        let total: f32 = self
            .inputs
            .iter()
            .map(|(_, quantity)| *quantity as f32)
            .sum();

        RARITIES
            .into_iter()
            .map(|query| {
                self.inputs
                    .iter()
                    .zip(rarities)
                    .map(|((_, quantity), rarity)| {
                        let mut output = 0.0;
                        if query == *rarity {
                            output += 0.8 * (1.0 - rank_up)
                        }
                        if query == rarity.next() {
                            output += 0.2 * (1.0 - rank_up) + 0.8 * rank_up
                        }
                        if query == rarity.next().next() {
                            output += 0.2 * rank_up
                        }
                        output * *quantity as f32
                    })
                    .sum::<f32>()
                    / total
            })
            .collect::<Vec<_>>()
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

    pub fn remove(&mut self, stack: ItemStack) -> bool {
        let Some(quantity) = self.0.get_mut(&stack.item) else {
            return false;
        };

        if *quantity == stack.quantity {
            self.0.remove(&stack.item);
        } else {
            *quantity -= stack.quantity
        }
        true
    }

    pub fn get(&self, item: Item) -> Option<usize> {
        self.0.get(&item).copied()
    }

    pub fn set(&mut self, stack: ItemStack) {
        if stack.quantity == 0 {
            self.0.remove(&stack.item);
            return;
        }

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

#[derive(Clone)]
pub struct LootTable<T> {
    entries: Vec<(f32, T)>,
}

impl<T> Default for LootTable<T> {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
        }
    }
}

impl<T> LootTable<T> {
    pub fn add(mut self, probability: f32, loot: T) -> Self {
        self.entries.push((probability, loot));
        self
    }

    pub fn pick(&self) -> &T {
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
