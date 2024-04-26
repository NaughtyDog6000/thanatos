use glam::{Vec2, Vec4};
use styx::{components::{Container, Offset, Text, VAlign, VGroup}, Signal};

use crate::{renderer::{Anchor, Ui}, window::{Keybind, Keyboard}, World};

pub struct Interactable {
    pub priority: f32,
    pub text: String,
    pub signal: Signal,
}

impl Interactable {
    pub fn new<T: ToString>(world: &World, text: T) -> Self {
        let mut ui = world.get_mut::<Ui>().unwrap();
        let signal = ui.signals.signal();
        Interactable {
            priority: f32::MAX,
            text: text.to_string(),
            signal
        }
    }
}

fn interact_ui(world: &World) {
    let interactables = world.query::<&Interactable>();
    let Some(interactable) = interactables.iter().min_by(|a, b| a.priority.partial_cmp(&b.priority).unwrap()) else { return };
    if interactable.priority == f32::MAX { return; }

    let mut ui = world.get_mut::<Ui>().unwrap();
    let font = ui.font.clone();

    ui.add(
        Anchor::Cursor,
        Offset {
            offset: Vec2::new(32.0, 32.0),
            child: VGroup::new(VAlign::Center, 4.0)
                .add(Container {
                    padding: 4.0,
                    colour: Vec4::new(0.2, 0.2, 0.2, 1.0),
                    radius: 4.0,
                    child: Text {
                        font: font.clone(),
                        text: String::from("F"),
                        font_size: 24.0,
                        colour: Vec4::ONE,
                    },
                })
                .add(Container {
                    padding: 4.0,
                    colour: Vec4::new(0.1, 0.1, 0.1, 1.0),
                    radius: 4.0,
                    child: Text {
                        font: font.clone(),
                        text: interactable.text.clone(),
                        font_size: 16.0,
                        colour: Vec4::ONE,
                    },
                }),
        },
    );

    let keyboard = world.get::<Keyboard>().unwrap();
    if keyboard.is_down(Keybind::Interact) {
        ui.signals.set(interactable.signal); 
    }
}

pub fn add(world: World) -> World {
    world.with_ticker(interact_ui)
}
