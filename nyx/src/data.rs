use crate::item::{Item, ItemStack, LootTable, Recipe};

pub fn recipes() -> Vec<Recipe> {
    vec![
        Recipe {
            inputs: vec![ItemStack {
                item: Item::CopperOre,
                quantity: 2,
            }],
            outputs: vec![ItemStack {
                item: Item::CopperIngot,
                quantity: 1,
            }],
        },
        Recipe {
            inputs: vec![ItemStack {
                item: Item::CopperIngot,
                quantity: 3,
            }],
            outputs: vec![ItemStack {
                item: Item::CopperSword,
                quantity: 1,
            }],
        },
    ]
}

pub fn nodes() -> Vec<LootTable> {
    vec![LootTable::default().add(
        1.0,
        vec![ItemStack {
            item: Item::CopperOre,
            quantity: 2,
        }],
    )]
}
