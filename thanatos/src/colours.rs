use glam::Vec4;
use nyx::item::Rarity;

pub const COMMON: Vec4 = Vec4::ONE;
pub const UNCOMMON: Vec4 = Vec4::new(0.133, 0.773, 0.369, 1.0);
pub const RARE: Vec4 = Vec4::new(0.024, 0.714, 0.831, 1.0);
pub const EPIC: Vec4 = Vec4::new(0.659, 0.333, 0.969, 1.0);
pub const LEGENDARY: Vec4 = Vec4::new(0.918, 0.345, 0.047, 1.0);

pub fn rarity_colour(rarity: Rarity) -> Vec4 {
    match rarity {
        Rarity::Common => COMMON,
        Rarity::Uncommon => UNCOMMON,
        Rarity::Rare => RARE,
        Rarity::Epic => EPIC,
        Rarity::Legendary => LEGENDARY
    }
}
