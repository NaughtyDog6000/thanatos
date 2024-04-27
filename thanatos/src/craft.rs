use glam::{Vec2, Vec4};
use nyx::{
    data,
    item::{Inventory, Item, ItemStack, Rarity, Recipe, RecipeOutput, RARITIES},
    protocol::Serverbound,
};
use styx::{
    components::{
        text, Clicked, Constrain, Container, Gap, HAlign, HGroup, Text, VAlign, VGroup, VPair,
    },
    Constraint, Signal,
};
use tecs::{System, SystemMut};

use crate::{
    colours::rarity_colour,
    event::Event,
    net::Connection,
    renderer::{Anchor, Ui},
    window::Keyboard,
    World,
};

pub struct CraftUi {
    open: bool,
    craft: Signal,
    recipe: Option<usize>,
    recipes: Vec<(Signal, Recipe)>,
    inputs: Vec<(Rarity, Vec<Signal>)>,
}

impl CraftUi {
    pub fn new(world: &World, recipes: &[Recipe]) -> Self {
        let mut ui = world.get_mut::<Ui>().unwrap();
        let recipes = recipes
            .iter()
            .map(|recipe| (ui.signals.signal(), recipe.clone()))
            .collect();
        let craft = ui.signals.signal();
        Self {
            open: false,
            craft,
            recipe: None,
            recipes,
            inputs: Vec::new(),
        }
    }
}

impl SystemMut<Event> for CraftUi {
    fn tick(&mut self, world: &World) {
        let keyboard = world.get::<Keyboard>().unwrap();
        if keyboard.pressed("c") {
            self.open = !self.open;
        }

        if !self.open {
            return;
        }

        let mut ui = world.get_mut::<Ui>().unwrap();
        let inventory = world.get_mut::<Inventory>().unwrap();

        let mut view = VGroup::new(VAlign::Top, 32.0);

        let recipes = self.recipes.iter().enumerate().fold(
            HGroup::new(HAlign::Left, 16.0),
            |component, (i, (signal, recipe))| {
                if ui.signals.get(*signal) {
                    self.recipe = Some(i);
                    self.inputs = (0..self.recipes[i].1.inputs.len())
                        .map(|_| {
                            (
                                Rarity::Common,
                                (0..5).map(|_| ui.signals.signal()).collect(),
                            )
                        })
                        .collect()
                }

                let text = match recipe.output {
                    RecipeOutput::Items(kind, quantity) => {
                        format!("{kind} x {quantity}")
                    }
                    RecipeOutput::Equipment(equipment) => equipment.to_string(),
                };

                component.add(Clicked {
                    signal: *signal,
                    child: Text {
                        text,
                        font_size: 48.0,
                        font: ui.font.clone(),
                        colour: Vec4::ONE,
                    },
                })
            },
        );

        let recipes = Container {
            child: recipes,
            padding: 32.0,
            colour: Vec4::new(0.1, 0.1, 0.1, 1.0),
            radius: 8.0,
        };

        view = view.add(recipes);

        if let Some(index) = &self.recipe {
            let recipe = &self.recipes.get(*index).unwrap().1;

            let inputs = recipe.inputs.iter().cloned().zip(&mut self.inputs).fold(
                HGroup::new(HAlign::Left, 16.0).add(text("Inputs:", 48.0, ui.font.clone())),
                |inputs, ((kind, quantity), (rarity, signals))| {
                    RARITIES
                        .into_iter()
                        .zip(signals.clone())
                        .for_each(|(r, signal)| {
                            if ui.signals.get(signal) {
                                *rarity = r
                            }
                        });

                    let left = Text {
                        text: format!("{} x {}", kind, quantity),
                        font_size: 48.0,
                        font: ui.font.clone(),
                        colour: rarity_colour(*rarity),
                    };

                    let right = RARITIES.into_iter().zip(signals).fold(
                        VGroup::new(VAlign::Top, 16.0),
                        |right, (rarity, signal)| {
                            let quantity = inventory.get(Item { kind, rarity }).unwrap_or_default();
                            right.add(Clicked {
                                signal: *signal,
                                child: Text {
                                    text: quantity.to_string(),
                                    font_size: 48.0,
                                    font: ui.font.clone(),
                                    colour: rarity_colour(rarity),
                                },
                            })
                        },
                    );

                    inputs.add(VPair::new(left, right, Gap::Auto))
                },
            );

            let output = HGroup::new(HAlign::Left, 16.0)
                .add(text("Output:", 48.0, ui.font.clone()))
                .add({
                    let text = match recipe.output {
                        RecipeOutput::Items(kind, quantity) => {
                            format!("{kind} x {quantity}")
                        }
                        RecipeOutput::Equipment(equipment) => equipment.to_string(),
                    };

                    Text {
                        text,
                        font_size: 48.0,
                        font: ui.font.clone(),
                        colour: Vec4::ONE,
                    }
                })
                .add({
                    let chances = recipe.rarity_chances(
                        &self
                            .inputs
                            .iter()
                            .map(|(rarity, _)| *rarity)
                            .collect::<Vec<_>>(),
                    );
                    RARITIES.into_iter().zip(chances).fold(
                        VGroup::new(VAlign::Top, 16.0),
                        |chances, (rarity, chance)| {
                            chances.add(Text {
                                text: format!("{}%", (chance * 100.0) as u32),
                                font_size: 48.0,
                                colour: rarity_colour(rarity),
                                font: ui.font.clone(),
                            })
                        },
                    )
                });

            let button = Clicked {
                signal: self.craft,
                child: Container {
                    padding: 32.0,
                    colour: Vec4::new(0.2, 0.2, 0.2, 1.0),
                    radius: 8.0,
                    child: text("Craft", 48.0, ui.font.clone()),
                },
            };

            let recipe = HGroup::new(HAlign::Left, 96.0)
                .add(inputs)
                .add(output)
                .add(button);
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

        if let Some(index) = &self.recipe {
            let recipe = &self.recipes.get(*index).unwrap().1;
            let rarities = self
                .inputs
                .iter()
                .map(|(rarity, _)| *rarity)
                .collect::<Vec<_>>();
            if recipe.craftable(&inventory.items().collect::<Vec<_>>(), &rarities)
                && ui.signals.get(self.craft)
            {
                let mut conn = world.get_mut::<Connection>().unwrap();
                conn.write(Serverbound::Craft(*index, rarities)).unwrap();
            }
        }

        ui.add(Anchor::Center, view);
    }
}

pub fn add(world: World) -> World {
    let ui = CraftUi::new(&world, &data::recipes());
    world.with_system_mut(ui)
}
