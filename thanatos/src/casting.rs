use std::fmt;

use gltf::Material;
use serde::{Deserialize, Serialize};
use styx::{
    components::{Clicked, HAlign, HGroup, Text, VAlign, VGroup},
    Signal,
};
use tecs::{Is, SystemMut};

use crate::{
    combat::CombatOffensive,
    event::Event,
    player::Player,
    renderer::{Anchor, Ui},
    targeting::SelectedEntity,
    uiutils, World,
};

// casting UI for the player
#[derive(Debug, Clone)]
pub struct CastingUI {
    pub open: bool,
    pub casts: Vec<Signal>,
}

// MAX EQUIPED SKILLS
const MAX_EQUIPED: usize = 20;

pub fn add(world: World) -> World {
    let ui = CastingUI::new(&world);
    world.with_system_mut(ui)
}

impl CastingUI {
    fn new(world: &World) -> Self {
        let mut ui = world.get_mut::<Ui>().unwrap();

        Self {
            open: true,
            casts: vec![ui.signals.signal(); MAX_EQUIPED],
        }
    }
}

impl SystemMut<Event> for CastingUI {
    fn tick(&mut self, world: &World) {
        if !self.open {
            return;
        }
        let mut ui = world.get_mut::<Ui>().unwrap();

        // Get the equiped skills for the player
        let (mut selected, player_offensive, _) =
            world.query_one::<(&mut SelectedEntity, &CombatOffensive, Is<Player>)>();

        let equiped = &player_offensive.equiped_skills;

        let mut skill_slot_ui = VGroup::new(VAlign::Top, 32.0);
        // add the equiped skills to the view
        for (i, skill) in equiped.iter().enumerate() {
            let mut skill_ui = HGroup::new(HAlign::Left, 32.0);
            skill_ui = skill_ui.add(Text {
                text: skill.name.clone(),
                font: ui.font.clone(),
                font_size: 32.0,
                colour: crate::colours::rarity_colour(nyx::item::Rarity::Legendary),
            });

            skill_ui = skill_ui.add(Text {
                text: uiutils::progress_bar_string(15, 0.25),
                font: ui.font.clone(),
                font_size: 16.0,
                colour: crate::colours::rarity_colour(nyx::item::Rarity::Epic),
            });

            // create a signal for each equiped skill to cast
            skill_ui = skill_ui.add(Clicked {
                signal: self.casts[i],
                child: Text {
                    text: "cast".to_string(),
                    font_size: 48.0,
                    font: ui.font.clone(),
                    colour: glam::Vec4::ONE,
                },
            });

            skill_slot_ui = skill_slot_ui.add(skill_ui);
        }

        ui.add(Anchor::BottomCenter, skill_slot_ui);

        // TODO! DEBUG REMOVE
        for i in 0..equiped.len() {
            if ui.signals.get(self.casts[i]) {
                println!("cast skill {}", i);
            }
        }
    }
}

// skills are something that the user actively "casts" or activates

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Skill {
    pub name: String,
    pub description: String,
    pub cooldown: f32, // seconds

    pub targeting_method: SkillTargeting,
    pub cast_type: CastType,

    // things the skill does (damages an entitiy, applies a slow, poisons them etc)
    pub effects: Vec<Effect>,
}

impl fmt::Display for Skill {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Name: {}, Description: {}, Cooldown: {:.2}s, Targeting Method: {}, Cast Type: {}, Effects: {:?}",
            self.name,
            self.description,
            self.cooldown,
            self.targeting_method,
            self.cast_type,
            self.effects
        )
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy)]
pub enum SkillTargeting {
    Point {
        range: f32,
    }, // a position in the world
    Entity {
        range: f32,
        can_target_friendly: bool,
        can_target_enemies: bool,
        can_target_self: bool,
    }, // casting on something (player, enemy, structure)
    None, // press button to cast / targets self
}

impl fmt::Display for SkillTargeting {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SkillTargeting::Point { range } => write!(f, "Point (Range: {:.2})", range),
            SkillTargeting::Entity {
                range,
                can_target_friendly,
                can_target_enemies,
                can_target_self,
            } => write!(
                f,
                "Entity (Range: {:.2}, Can Target Friendly: {}, Can Target Enemies: {}, Can Target Self: {})",
                range, can_target_friendly, can_target_enemies, can_target_self
            ),
            SkillTargeting::None => write!(f, "None"),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub enum CastType {
    Instant,
    Charge {
        charge_duration: f32,  // how long from cast start to effects being fired
        stationary_cast: bool, // if the player must be still while casting
    },
    // Toggle, //debateable if should be included
}

impl fmt::Display for CastType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CastType::Instant => write!(f, "Instant"),
            CastType::Charge {
                charge_duration,
                stationary_cast,
            } => write!(
                f,
                "Charge (Duration: {:.2}, Stationary Cast: {})",
                charge_duration, stationary_cast
            ),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub enum EffectType {
    Damage(DamageEffect), //damage types and stats
    DOT(DamageOverTimeEffect),
    BufDebuf,                 //+20% haste, rage etc (temporary changes in power)
    Healing(HealEffect),      // instant/over time, amount, percent/aboslute
    Movement(MovementEffect), // teleports the player or dashes to a position etc
}

impl fmt::Display for EffectType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EffectType::Damage(damage_effect) => write!(f, "Damage: {}", damage_effect),
            EffectType::DOT(dot_effect) => write!(f, "DOT: {}", dot_effect),
            EffectType::BufDebuf => write!(f, "Buff/Debuff"),
            EffectType::Healing(heal_effect) => write!(f, "Healing: {}", heal_effect),
            EffectType::Movement(movement_effect) => write!(f, "Movement: {}", movement_effect),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct Effect {
    pub area_of_effect: Option<f32>, // for AOE / spash damage Effects
    pub variant: EffectType,
}

impl fmt::Display for Effect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.area_of_effect {
            Some(aoe) => write!(f, "Area of Effect: {:.2}, Type: {}", aoe, self.variant),
            None => write!(f, "Area of Effect: None, Type: {}", self.variant),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct DamageEffect {
    pub true_damage: u32, // damage that ignores all resistances
    pub melee_damage: u32,
    pub ranged_damage: u32,
    pub magic_damage: u32,
}

impl fmt::Display for DamageEffect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "True Damage: {}, Melee Damage: {}, Ranged Damage: {}, Magic Damage: {}",
            self.true_damage, self.melee_damage, self.ranged_damage, self.magic_damage
        )
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct HealEffect {}

impl fmt::Display for HealEffect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Heal Effect NOT IMPLEMENTED!")
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct MovementEffect {
    pub movement_distance: f32,      // where to move towards
    pub movement_duration: f32,      // how long to go from A->B (could be changed to speed)
    pub untargetable: bool, // if the user should be untargetable during this (dodge roll from darksouls)
    pub can_travel_unwalkable: bool, // if the path can go over an otherwise un-walkable surface (void/dashing over lava)
}

impl fmt::Display for MovementEffect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Movement: Distance: {}, Duration: {}, Untargetable: {}, Can Travel Unwalkable: {}",
            self.movement_distance,
            self.movement_duration,
            self.untargetable,
            self.can_travel_unwalkable
        )
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct DamageOverTimeEffect {
    pub damage_type: DamageOverTimeType,
    pub duration: f32,
    pub damage_per_tick: u32,
}

impl fmt::Display for DamageOverTimeEffect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Damage Over Time: Type: {:?}, Duration: {}, Damage Per Tick: {}",
            self.damage_type, self.duration, self.damage_per_tick
        )
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub enum DamageOverTimeType {
    Poison,
    Burn,
    Freeze,
}

impl fmt::Display for DamageOverTimeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DamageOverTimeType::Poison => write!(f, "Poison"),
            DamageOverTimeType::Burn => write!(f, "Burn"),
            DamageOverTimeType::Freeze => write!(f, "Freeze"),
        }
    }
}
