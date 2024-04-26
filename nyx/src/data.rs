use crate::{
    equipment::EquipmentKind,
    item::{Item, ItemKind, ItemStack, LootTable, Rarity, Recipe, RecipeOutput},
};

pub fn recipes() -> Vec<Recipe> {
    vec![
        Recipe {
            inputs: vec![(ItemKind::CopperOre, 2)],
            outputs: vec![RecipeOutput::Items(ItemKind::CopperIngot, 1)],
        },
        Recipe {
            inputs: vec![(ItemKind::CopperIngot, 3)],
            outputs: vec![RecipeOutput::Equipment(EquipmentKind::CopperSword)],
        },
    ]
}

pub fn nodes() -> Vec<LootTable> {
    vec![LootTable::default().add(
        1.0,
        vec![ItemStack {
            item: Item {
                kind: ItemKind::CopperOre,
                rarity: Rarity::Common,
            },
            quantity: 2,
        }],
    )]
}
