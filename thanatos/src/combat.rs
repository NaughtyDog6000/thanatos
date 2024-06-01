use glam::{Quat, Vec3, Vec4};
use log::{error, info, trace, warn};
use serde::{Deserialize, Serialize};
use tecs::{EntityId, Is};

use crate::{
    camera::Camera,
    player::Player,
    renderer::RenderObject,
    targeting::{Selectable, SelectedEntity},
    transform::Transform,
    window::Keyboard,
    TargetDummy, World,
};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct AttackType {
    pub damage: u32,
    pub penetration: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CombatOffensive {
    pub fire: AttackType,
    pub earth: AttackType,
    pub lightning: AttackType,
    pub air: AttackType,
    pub nature: AttackType,

    pub true_damage: u32,
}

impl std::fmt::Display for CombatOffensive {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "CombatOffensive: \nfire: {:?}\nearth: {:?}\nlightning: {:?}\nair: {:?}\nnature: {:?}\ntrue_damage: {:?}",
            self.fire, self.earth, self.lightning, self.air, self.nature, self.true_damage
        )
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CombatDefensive {
    pub health: u32,
    pub max_health: u32,
    pub fire_resistance: u32,
    pub earth_resistance: u32,
    pub lightning_resistance: u32,
    pub air_resistance: u32,
    pub nature_resistance: u32,
}

impl std::fmt::Display for CombatDefensive {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "CombatDefensive: \nhealth: {}\nfire_resistance: {}\nearth_resistance: {}\nlightning_resistance: {}\nair_resistance: {}\nnature_resistance: {}\n",
            self.health, self.fire_resistance, self.earth_resistance, self.lightning_resistance, self.air_resistance, self.nature_resistance
        )
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct AttackOutcome {
    pub fire_damage: u32,
    pub earth_damage: u32,
    pub lightning_damage: u32,
    pub air_damage: u32,
    pub nature_damage: u32,

    pub true_damage: u32,

    pub post_attack_health: u32,
}

impl AttackOutcome {
    pub fn sum_damage(&self) -> u32 {
        return self.fire_damage
            + self.earth_damage
            + self.lightning_damage
            + self.air_damage
            + self.nature_damage
            + self.true_damage;
    }
}

pub trait Attackable {
    /// returns the new health of the attacked entity along with how much damge was done.
    /// If the damage would put the player bellow 0, the health will be set to 0.
    fn receive_attack(&mut self, damage_source: &CombatOffensive) -> AttackOutcome;
}

impl Attackable for CombatDefensive {
    fn receive_attack(&mut self, source: &CombatOffensive) -> AttackOutcome {
        let fire_damage: u32;
        if source.fire.penetration > self.fire_resistance {
            fire_damage = source.fire.damage;
        } else {
            fire_damage =
                source.fire.damage * 100 / (100 + (self.fire_resistance - source.fire.penetration));
        }

        // repeat for each type of damage

        let earth_damage: u32;
        if source.earth.penetration > self.earth_resistance {
            earth_damage = source.earth.damage;
        } else {
            earth_damage = source.earth.damage * 100
                / (100 + (self.earth_resistance - source.earth.penetration));
        }

        let lightning_damage: u32;
        if source.lightning.penetration > self.lightning_resistance {
            lightning_damage = source.lightning.damage;
        } else {
            lightning_damage = source.lightning.damage * 100
                / (100 + (self.lightning_resistance - source.lightning.penetration));
        }

        let air_damage: u32;
        if source.air.penetration > self.air_resistance {
            air_damage = source.air.damage;
        } else {
            air_damage =
                source.air.damage * 100 / (100 + (self.air_resistance - source.air.penetration));
        }

        let nature_damage: u32;
        if source.nature.penetration > self.nature_resistance {
            nature_damage = source.nature.damage;
        } else {
            nature_damage = source.nature.damage * 100
                / (100 + (self.nature_resistance - source.nature.penetration));
        }

        let total_damage = fire_damage
            + earth_damage
            + lightning_damage
            + air_damage
            + nature_damage
            + source.true_damage;

        // to prevent underflows check if the total damage is greater than health and if so set to 0
        if total_damage > self.health {
            self.health = 0;
        } else {
            self.health -= total_damage;
        }

        return AttackOutcome {
            fire_damage,
            earth_damage,
            lightning_damage,
            air_damage,
            nature_damage,

            true_damage: source.true_damage,
            post_attack_health: self.health,
        };
    }
}

pub fn tick(world: &World) {
    let keyboard = world.get::<Keyboard>().unwrap();

    let (player_offensive, mut targeted, _) =
        world.query_one::<(&CombatOffensive, &mut SelectedEntity, Is<Player>)>();

    // TODO! BROKEN WHEN PRESS Z AFTER DUMMY DIES
    // attack every entity that is a target dummy
    if keyboard.pressed("z") {
        // let (player_offensive, _) = world.query_one::<(&CombatOffensive, Is<Player>)>();

        let (dummy_ids, _) = world.query::<(EntityId, Is<crate::TargetDummy>)>();
        for (index, id) in dummy_ids.iter().enumerate() {
            let mut defense_struct = world
                .get_component_mut::<crate::combat::CombatDefensive>(*id)
                .unwrap();
            let outcome = defense_struct.receive_attack(&player_offensive);
            info!("Outcome from attack: {:?}", outcome);

            if outcome.post_attack_health == 0 {
                // print to console including position
                let dummy_transfrom = world.get_component::<Transform>(*id).unwrap();
                info!(
                    "Entity {} died at ({}, {}, {})",
                    index,
                    dummy_transfrom.translation.x,
                    dummy_transfrom.translation.y,
                    dummy_transfrom.translation.z
                );

                *targeted = SelectedEntity::None;
                world.despawn::<crate::TargetDummy>(*id);
            }
        }
    }

    // attack the targeted entity
    if keyboard.pressed("x") {
        match *targeted {
            SelectedEntity::None => warn!("No enemy Targeted"),
            SelectedEntity::EntityId(targeted_id) => {
                let outcome = world
                    .get_component_mut::<CombatDefensive>(targeted_id)
                    .unwrap()
                    .receive_attack(&player_offensive);
                info!("Outcome from attack: {:?}", outcome);

                if outcome.post_attack_health == 0 {
                    // the requirement to pass in the type is a little annoying as this should work for any entity that implements Attackable
                    world.despawn::<TargetDummy>(targeted_id);
                    *targeted = SelectedEntity::None;
                }
            }
            SelectedEntity::Position(_) => {
                error!("area of effect/non entity targeting not implemented")
            }
        }
    }
}
