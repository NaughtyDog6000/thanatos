use crate::{
    equipment::EquipmentKind,
    item::{ItemKind, Recipe, RecipeOutput},
};

pub fn recipes() -> Vec<Recipe> {
    vec![
        Recipe {
            inputs: vec![(ItemKind::CopperOre, 2)],
            output: RecipeOutput::Item(ItemKind::CopperIngot),
        },
        Recipe {
            inputs: vec![(ItemKind::CopperIngot, 3)],
            output: RecipeOutput::Equipment(EquipmentKind::CopperSword),
        },
        Recipe {
            inputs: vec![(ItemKind::CopperIngot, 2)],
            output: RecipeOutput::Item(ItemKind::FireDamageReagent),
        },
    ]
}

pub mod nodes {
    use crate::item::{Item, ItemKind, ItemStack, LootTable, Rarity};

    pub const COPPER_ORE: usize = 0;

    pub fn get() -> Vec<LootTable<Vec<ItemStack>>> {
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
}

pub mod tasks {
    use crate::{
        item::Tag,
        task::{Query, Reward, Task},
    };

    pub const COPPER_SMELTING_1: usize = 0;
    pub const COPPER_MINING_1: usize = 1;

    pub fn tasks() -> Vec<Task> {
        vec![
            Task {
                query: Query {
                    tags: vec![Tag::Weaponsmithing, Tag::Copper],
                },
                required: 10,
                rewards: vec![
                    Reward::Proficiency(
                        Query {
                            tags: vec![Tag::Weaponsmithing],
                        },
                        0.01,
                    ),
                    Reward::Proficiency(
                        Query {
                            tags: vec![Tag::Weaponsmithing, Tag::Copper],
                        },
                        0.1,
                    ),
                ],
            },
            Task {
                query: Query {
                    tags: vec![Tag::Mining, Tag::Copper],
                },
                required: 10,
                rewards: vec![
                    Reward::Proficiency(
                        Query {
                            tags: vec![Tag::Mining],
                        },
                        0.01,
                    ),
                    Reward::Proficiency(
                        Query {
                            tags: vec![Tag::Mining, Tag::Copper],
                        },
                        0.1,
                    ),
                ],
            },
        ]
    }
}
