use tecs::{EntityId, Is};

use crate::{camera::Camera, player::Player, transform::Transform, window::Keyboard, TargetDummy, World};

#[derive(Debug)]
pub struct AttackType {
    pub damage: u32,
    pub penetration: u32,
}

#[derive(Debug)]
pub struct CombatOffensive {
    pub fire: AttackType,
    pub earth: AttackType,
    pub lightning: AttackType,
    pub air: AttackType,
    pub nature: AttackType,

    pub true_damage: u32,
}


#[derive(Debug)]
pub struct CombatDefensive {
    pub health: u32,
    pub fire_resistance: u32,
    pub earth_resistance: u32,
    pub lightning_resistance: u32,
    pub air_resistance: u32,
    pub nature_resistance: u32,

}
#[derive(Debug, PartialEq, Eq)]
pub struct AttackOutcome {
    pub fire_damage: u32,
    pub earth_damage: u32,
    pub lightning_damage: u32,
    pub air_damage: u32,
    pub nature_damage: u32,
    
    pub true_damage: u32,
    
    pub post_attack_health: u32
}

impl AttackOutcome {
    fn sum_damage(&self) -> u32 {
        return self.fire_damage + self.earth_damage + self.lightning_damage + self.air_damage + self.nature_damage + self.true_damage;
    }
}

pub trait Attack {
    /// returns the new health of the attacked entity along with how much damge was done.
    fn receive_attack(&mut self, damage_source: &CombatOffensive) -> AttackOutcome;
}

impl Attack for CombatDefensive {
    fn receive_attack(&mut self, source: &CombatOffensive) -> AttackOutcome {

        let fire_damage: u32;
        if source.fire.penetration > self.fire_resistance {
            fire_damage = source.fire.damage;
        } else {
            fire_damage = source.fire.damage * 100 / (100 + (self.fire_resistance - source.fire.penetration));
        }
        
        // repeat for each type of damage

        let earth_damage: u32;
        if source.earth.penetration > self.earth_resistance {
            earth_damage = source.earth.damage;
        } else {
            earth_damage = source.earth.damage * 100 / (100 + (self.earth_resistance - source.earth.penetration));
        }

        let lightning_damage: u32;
        if source.lightning.penetration > self.lightning_resistance {
            lightning_damage = source.lightning.damage;
        } else {
            lightning_damage = source.lightning.damage * 100 / (100 + (self.lightning_resistance - source.lightning.penetration));
        }

        let air_damage: u32;
        if source.air.penetration > self.air_resistance {
            air_damage = source.air.damage;
        } else {
            air_damage = source.air.damage * 100 / (100 + (self.air_resistance - source.air.penetration));
        }

        let nature_damage: u32;
        if source.nature.penetration > self.nature_resistance {
            nature_damage = source.nature.damage;
        } else {
            nature_damage = source.nature.damage * 100 / (100 + (self.nature_resistance - source.nature.penetration));
        }

        let total_damage = fire_damage + earth_damage + lightning_damage + air_damage + nature_damage + source.true_damage;

        self.health -= total_damage;


        return AttackOutcome {
            fire_damage,
            earth_damage,
            lightning_damage,
            air_damage,
            nature_damage,
            
            true_damage: source.true_damage,
            post_attack_health: self.health
        };
    }   

}


pub fn tick(world: &World) {
    let keyboard = world.get::<Keyboard>().unwrap();
    let mut camera = world.get_mut::<Camera>().unwrap();

    let (mut transform, player_offensive, _) = world.query_one::<(&mut Transform, &CombatOffensive, Is<Player>)>();

    // combat debug stuff
    if keyboard.is_down("z") {
        // let (player_offensive, _) = world.query_one::<(&CombatOffensive, Is<Player>)>();
        

        let (dummy_ids, _) = world.query::<(EntityId, Is<crate::TargetDummy>)>();
        for id in dummy_ids {
            let mut defense_struct = world.get_component_mut::<crate::combat::CombatDefensive>(id).unwrap();

            let outcome = defense_struct.receive_attack(&player_offensive);
            println!("Outcome from attack: {:?}", outcome);

        }
    }

    let mouse = world.get_mut::<crate::window::Mouse>().unwrap();
    let window = world.get::<crate::window::Window>().unwrap();

    // try and select via clicking on entity
    if mouse.is_down(winit::event::MouseButton::Left) {
        let world_pos = camera.ndc_to_world(window.screen_to_ndc(mouse.position));
        let ray = crate::collider::Ray::from_points(camera.eye(), world_pos);
        // let mut targeted = world.get_component_mut::<TargetedEntity>(player).unwrap();

        let (ids, colliders, _) = world.query::<(EntityId, &crate::collider::Collider, Is<TargetDummy>)>();

        if ids.len() == 0 { return; }

        let mut clicked_on: &EntityId; 
        for (ind, collider) in colliders.iter().enumerate() {
            if collider.intersects(ray).is_some() {
                clicked_on = &ids[ind];
                println!("target selected!");

            }
        }


    }
}