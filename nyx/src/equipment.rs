use std::fmt::Display;

use crate::item::Rarity;

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub enum EquipmentKind {
    CopperSword,
}

impl Display for EquipmentKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            Self::CopperSword => "Copper Sword"
        })
    }
}

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub enum Passive {
    TestPassive,
}

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct EquipmentId(pub u64);

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Equipment {
    pub id: EquipmentId,
    pub kind: EquipmentKind,
    pub rarity: Rarity,
    pub durability: u32,
    pub passives: Vec<Passive>,
}

pub struct EquipmentInventory(pub Vec<Equipment>);

#[derive(Default)]
pub struct Equipped {
    pub weapon: Option<EquipmentId>,
}

impl Equipped {
    pub fn equipment(&self) -> impl Iterator<Item = EquipmentId> + '_ {
        [self.weapon.as_ref()].into_iter().filter_map(|x| x).copied()
    }
}
