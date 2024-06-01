use glam::{Vec3, Vec4};
use log::{error, info, trace};
use serde::{Deserialize, Serialize};
use styx::components::{HAlign, HGroup, Text, VAlign, VGroup};
use tecs::{EntityId, Is, SystemMut};

use crate::{
    assets::Material,
    camera::Camera,
    combat::{CombatDefensive, CombatOffensive},
    event::Event,
    player::Player,
    renderer::RenderObject,
    uiutils::progress_bar_string,
    TargetDummy, World,
};

#[derive(Clone, Default)]
pub enum SelectedEntity {
    #[default]
    None,
    EntityId(EntityId),
    Position(Vec3),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Selectable {
    // the material that will be used when the entity is selected
    pub selected_material: Material,
    // the default material
    pub unselected_material: Material,

    pub selected_name: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectedUIData {
    pub name: Option<String>,
    pub offensive_stats: Option<CombatOffensive>,
    pub defensive_stats: Option<CombatDefensive>,
}

impl Default for SelectedUIData {
    fn default() -> Self {
        Self {
            name: None,
            offensive_stats: None,
            defensive_stats: None,
        }
    }
}

const HEALTHBARWIDTH: usize = 40;

impl SelectedUIData {
    pub fn to_hgroup(&self, ui: &crate::renderer::Ui) -> HGroup {
        let mut hgroup = HGroup::new(HAlign::Center, 32.0);
        // if it has a name then display it
        if let Some(name) = &self.name {
            hgroup = hgroup.add(Text {
                text: name.clone(),
                font: ui.font.clone(),
                font_size: 32.0,
                colour: Vec4::ONE,
            });
        }
        // if it has defensive stats then display them and
        // display health as a progress bar
        if let Some(defensive_stats) = &self.defensive_stats {
            hgroup = hgroup.add(Text {
                text: progress_bar_string(
                    HEALTHBARWIDTH,
                    defensive_stats.health as f32 / defensive_stats.max_health as f32,
                ),
                font: ui.font.clone(),
                font_size: 16.0,
                colour: Vec4::ONE,
            });
            hgroup = hgroup.add(Text {
                text: defensive_stats.to_string(),
                font: ui.font.clone(),
                font_size: 16.0,
                colour: Vec4::ONE,
            });
        }
        // if it has offensive stats then display them
        if let Some(offensive_stats) = &self.offensive_stats {
            hgroup = hgroup.add(Text {
                text: offensive_stats.to_string(),
                font: ui.font.clone(),
                font_size: 16.0,
                colour: Vec4::ONE,
            });
        }

        return hgroup;
    }
}
pub struct SelectedUI {
    hide_always: bool,
}

pub fn add(world: World) -> World {
    let ui = SelectedUI { hide_always: false };
    world.with_system_mut(ui)
}

impl SystemMut<Event> for SelectedUI {
    fn tick(&mut self, world: &World) {
        if self.hide_always {
            return;
        }

        let (mut targeted, _) = world.query_one::<(&mut SelectedEntity, Is<Player>)>();

        let mut selectedUIData: SelectedUIData = Default::default();
        match *targeted {
            SelectedEntity::None => return,
            SelectedEntity::Position(_) => return,
            SelectedEntity::EntityId(selected) => {
                // Display the name of the selected entity
                let opt_selectable = world.get_component::<Selectable>(selected);
                if opt_selectable.is_some() {
                    selectedUIData.name = Some(opt_selectable.unwrap().selected_name.clone());
                }
                // Display the combat stats of the selected entity
                let opt_defensive_stats = world.get_component::<CombatDefensive>(selected);
                if opt_defensive_stats.is_some() {
                    selectedUIData.defensive_stats = Some(opt_defensive_stats.unwrap().clone());
                }
                let opt_offensive_stats = world.get_component::<CombatOffensive>(selected);
                if opt_offensive_stats.is_some() {
                    selectedUIData.offensive_stats = Some(opt_offensive_stats.unwrap().clone());
                }
            }
        }

        let mut ui = world.get_mut::<crate::renderer::Ui>().unwrap();

        let view = VGroup::new(VAlign::Top, 32.0)
            // The name of the selected entity
            .add(Text {
                text: "Selected:".to_string(),
                font: ui.font.clone(),
                font_size: 48.0,
                colour: Vec4::ONE,
            })
            // The combat stats of the selected entity
            .add(selectedUIData.to_hgroup(&ui));

        ui.add(crate::renderer::Anchor::TopLeft, view);
    }
}

pub fn tick(world: &World) {
    // try and select via clicking on entity
    let camera = world.get_mut::<Camera>().unwrap();
    let mouse = world.get_mut::<crate::window::Mouse>().unwrap();
    let window = world.get::<crate::window::Window>().unwrap();

    let (mut targeted, _) = world.query_one::<(&mut SelectedEntity, Is<Player>)>();

    if mouse.pressed(winit::event::MouseButton::Left) {
        let world_pos = camera.ndc_to_world(window.screen_to_ndc(mouse.position));
        let ray = crate::collider::Ray::from_points(camera.eye(), world_pos);

        // clear the previous target and reset its material
        match *targeted {
            SelectedEntity::None => (),
            SelectedEntity::EntityId(targeted_id) => {
                let mut render_object = world
                    .get_component_mut::<RenderObject>(targeted_id)
                    .unwrap();
                let selectable = world.get_component::<Selectable>(targeted_id).unwrap();
                *render_object.material.colour = *selectable.unselected_material.colour;
                trace!("target: {:?} cleared", targeted_id);
            }
            SelectedEntity::Position(_) => {
                error!("area of effect/non entity targeting not implemented");
            }
        }

        // get all the possible targets that can be selected
        let (ids, colliders, selectables, _) = world.query::<(
            EntityId,
            &crate::collider::Collider,
            &Selectable,
            Is<TargetDummy>,
        )>();

        trace!("targeting: {:?}", ids.len());
        if ids.len() == 0 {
            return;
        }

        let mut new_target_found = false;

        // check if the ray intersects with any of the colliders and if so select them
        // TODO: make this more efficient & handle multiple targets in one raycast
        for (ind, (collider, selectable)) in colliders.iter().zip(selectables.iter()).enumerate() {
            if collider.intersects(ray, world).is_some() {
                let mut render_object = world.get_component_mut::<RenderObject>(ids[ind]).unwrap();
                // set the rendered material of that entity to it's selected material
                *render_object.material.colour = *selectable.selected_material.colour;
                info!("target: {:?} selected", ids[ind]);
                // set as the targeted entity
                *targeted = SelectedEntity::EntityId(ids[ind]);
                new_target_found = true;
            }
        }

        if !new_target_found {
            trace!("no target found inside raycast, deselecting previous target");
            *targeted = SelectedEntity::None;
        }
    }
}
