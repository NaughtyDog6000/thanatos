use glam::{Vec2, Vec4};
use nyx::{
    equipment::{EquipmentId, EquipmentInventory, Equipped, Passive},
    item::{Inventory, Item, ItemStack},
    protocol::{Clientbound, Serverbound},
};
use styx::{
    components::{
        text, Clicked, Constrain, Container, HAlign, HGroup, RightClicked, Text, VAlign, VGroup,
    },
    Constraint, Signal,
};
use tecs::SystemMut;

use crate::{
    colours::rarity_colour,
    event::Event,
    net::Connection,
    renderer::{Anchor, Ui},
    window::Keyboard,
    World,
};

pub struct EquipmentUi {
    open: bool,
    refine: Signal,
    reagent: Option<Item>,
    reagents: Vec<(Item, Signal)>,
    refining: Option<EquipmentId>,
    signals: Vec<(EquipmentId, (Signal, Signal))>,
}

impl EquipmentUi {
    pub fn new(world: &World) -> Self {
        let mut ui = world.get_mut::<Ui>().unwrap();
        Self {
            refine: ui.signals.signal(),
            open: false,
            reagent: None,
            reagents: Vec::new(),
            refining: None,
            signals: Vec::new(),
        }
    }
}

impl SystemMut<Event> for EquipmentUi {
    fn tick(&mut self, world: &World) {
        let keyboard = world.get::<Keyboard>().unwrap();
        if keyboard.pressed("e") {
            self.open = !self.open;
        }

        if !self.open {
            return;
        }

        let mut ui = world.get_mut::<Ui>().unwrap();
        let mut equipped = world.get_mut::<Equipped>().unwrap();

        self.signals.drain(..).for_each(|(equipment, signal)| {
            if ui.signals.get(signal.0) {
                equipped.weapon = match equipped.weapon {
                    Some(current) if current == equipment => None,
                    _ => Some(equipment),
                }
            }

            if ui.signals.get(signal.1) {
                self.refining = Some(equipment);
            }
        });

        let equipment = world.get::<EquipmentInventory>().unwrap();
        let equipped = equipped.equipment().collect::<Vec<_>>();

        let list = equipment.0.iter().fold(
            HGroup::new(styx::components::HAlign::Left, 16.0),
            |list, equipable| {
                let mut colour = rarity_colour(equipable.rarity);
                if !equipped.iter().any(|id| equipable.id == *id) {
                    colour *= Vec4::new(0.5, 0.5, 0.5, 1.0)
                }

                let signals = (ui.signals.signal(), ui.signals.signal());
                self.signals.push((equipable.id, signals));

                let desc = HGroup::new(HAlign::Left, 8.0).add(Text {
                    text: format!("{}", equipable.kind),
                    font_size: 48.0,
                    font: ui.font.clone(),
                    colour,
                });
                let desc = equipable.passives.iter().fold(desc, |desc, passive| {
                    desc.add(Text {
                        text: passive.to_string(),
                        font_size: 24.0,
                        font: ui.font.clone(),
                        colour,
                    })
                });

                list.add(RightClicked {
                    signal: signals.1,
                    child: Clicked {
                        signal: signals.0,
                        child: desc,
                    },
                })
            },
        );

        let list = Container {
            padding: 32.0,
            colour: Vec4::new(0.1, 0.1, 0.1, 1.0),
            radius: 8.0,
            child: list,
        };

        let mut view = VGroup::new(VAlign::Top, 32.0).add(list);

        if let Some(input) = self.refining {
            self.reagents.drain(..).for_each(|(reagent, signal)| {
                if ui.signals.get(signal) {
                    self.reagent = Some(reagent)
                }
            });

            let inventory = world.get::<Inventory>().unwrap();
            let passives = inventory
                .items()
                .filter_map(|ItemStack { item, .. }| item.passive().map(|_| item))
                .fold(HGroup::new(HAlign::Left, 16.0), |reagents, reagent| {
                    let signal = ui.signals.signal();
                    self.reagents.push((reagent, signal));
                    reagents.add(Clicked {
                        signal,
                        child: Text {
                            text: reagent.passive().unwrap().to_string(),
                            font_size: 48.0,
                            colour: rarity_colour(reagent.rarity),
                            font: ui.font.clone(),
                        },
                    })
                });
            let passives = Container {
                padding: 32.0,
                colour: Vec4::new(0.1, 0.1, 0.1, 1.0),
                radius: 8.0,
                child: passives,
            };
            view = view.add(passives);

            if let Some(reagent) = self.reagent {
                let piece = equipment.0.iter().find(|piece| piece.id == input).unwrap();
                let inputs = piece.passives.iter().cloned().fold(
                    HGroup::new(HAlign::Left, 16.0).add(text("Current:", 48.0, ui.font.clone())),
                    |inputs, passive| {
                        inputs.add(Text {
                            text: passive.to_string(),
                            font_size: 48.0,
                            colour: Vec4::ONE,
                            font: ui.font.clone(),
                        })
                    },
                );
                let changing = piece.passives.iter().position(|p| *p == Passive::Empty);
                let outputs = piece.passives.iter().cloned().enumerate().fold(
                    HGroup::new(HAlign::Left, 16.0).add(text("After:", 48.0, ui.font.clone())),
                    |inputs, (i, mut p)| {
                        match changing {
                            Some(index) if i == index => p = reagent.passive().unwrap(),
                            _ => (),
                        }
                        inputs.add(Text {
                            text: p.to_string(),
                            font_size: 48.0,
                            colour: Vec4::ONE,
                            font: ui.font.clone(),
                        })
                    },
                );

                let button = Clicked {
                    signal: self.refine,
                    child: Container {
                        padding: 32.0,
                        colour: Vec4::new(0.2, 0.2, 0.2, 1.0),
                        radius: 8.0,
                        child: text("Refine", 48.0, ui.font.clone()),
                    },
                };

                if ui.signals.get(self.refine) {
                    let mut conn = world.get_mut::<Connection>().unwrap();
                    conn.write(Serverbound::Refine(input, reagent)).unwrap();
                    self.refining = None;
                    self.reagent = None;
                }

                let mut recipe = HGroup::new(HAlign::Left, 96.0)
                    .add(inputs)
                    .add(outputs);
                if changing.is_some() {
                    recipe = recipe.add(button);
                }

                let recipe = Container {
                    child: recipe,
                    padding: 32.0,
                    colour: Vec4::new(0.1, 0.1, 0.1, 1.0),
                    radius: 8.0,
                };
                let recipe = Constrain {
                    child: recipe,
                    constraint: Constraint {
                        min: Vec2::ZERO,
                        max: Vec2::new(800.0, 600.0),
                    },
                };
                view = view.add(recipe);
            }
        }

        ui.add(Anchor::Center, view);
    }
}

fn net(world: &World, event: &Event) {
    match event {
        Event::Recieved(Clientbound::AddEquipment(piece)) => {
            let mut equipment = world.get_mut::<EquipmentInventory>().unwrap();
            equipment.0.push(piece.clone());
        }
        Event::Recieved(Clientbound::SetPassives(id, passives)) => {
            let mut equipment = world.get_mut::<EquipmentInventory>().unwrap();
            equipment
                .0
                .iter_mut()
                .find(|piece| piece.id == *id)
                .map(|piece| piece.passives = passives.clone());
        }
        _ => (),
    }
}

pub fn add(world: World) -> World {
    let ui = EquipmentUi::new(&world);
    world
        .with_resource(Equipped::default())
        .with_resource(EquipmentInventory(Vec::new()))
        .with_system_mut(ui)
        .with_handler(net)
}
