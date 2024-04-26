use glam::Vec4;
use nyx::{
    data,
    item::{Inventory, Item, ItemStack, Rarity, Recipe, RecipeOutput},
    protocol::Serverbound,
};
use styx::{
    components::{text, Clicked, Container, HAlign, HGroup, Text, VAlign, VGroup},
    Signal,
};
use tecs::{System, SystemMut};

use crate::{
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
        let mut inventory = world.get_mut::<Inventory>().unwrap();

        let mut view = VGroup::new(VAlign::Top, 32.0);

        let recipes = self.recipes.iter().enumerate().fold(
            HGroup::new(HAlign::Left, 16.0),
            |component, (i, (signal, recipe))| {
                let output = recipe.outputs.first().unwrap();

                if ui.signals.get(*signal) {
                    self.recipe = Some(i)
                }

                let colour = if recipe.craftable(&inventory.items().collect::<Vec<_>>()) {
                    Vec4::new(0.0, 1.0, 0.0, 1.0)
                } else {
                    Vec4::new(1.0, 0.0, 0.0, 1.0)
                };

                let text = match output {
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
                        colour,
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

            let inputs = recipe.inputs.iter().cloned().fold(
                HGroup::new(HAlign::Left, 16.0).add(text("Inputs:", 48.0, ui.font.clone())),
                |inputs, (kind, quantity)| {
                    let colour = if quantity <= inventory.get(Item { kind, rarity: Rarity::Common }).unwrap_or_default()
                    {
                        Vec4::new(0.0, 1.0, 0.0, 1.0)
                    } else {
                        Vec4::new(1.0, 0.0, 0.0, 1.0)
                    };

                    inputs.add(Text {
                        text: format!("{} x {}", kind, quantity),
                        font_size: 48.0,
                        font: ui.font.clone(),
                        colour,
                    })
                },
            );

            let outputs = recipe.outputs.iter().fold(
                HGroup::new(HAlign::Left, 16.0).add(text("Outputs:", 48.0, ui.font.clone())),
                |outputs, output| {
                    let text = match output {
                        RecipeOutput::Items(kind, quantity) => {
                            format!("{kind} x {quantity}")
                        }
                        RecipeOutput::Equipment(equipment) => equipment.to_string(),
                    };

                    outputs.add(Text {
                        text,
                        font_size: 48.0,
                        font: ui.font.clone(),
                        colour: Vec4::ONE,
                    })
                },
            );

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
                .add(outputs)
                .add(button);
            let recipe = Container {
                child: recipe,
                padding: 32.0,
                colour: Vec4::new(0.1, 0.1, 0.1, 1.0),
                radius: 8.0,
            };
            view = view.add(recipe);
        }

        if let Some(index) = &self.recipe {
            let recipe = &self.recipes.get(*index).unwrap().1;
            if recipe.craftable(&inventory.items().collect::<Vec<_>>())
                && ui.signals.get(self.craft)
            {
                let mut conn = world.get_mut::<Connection>().unwrap();
                conn.write(Serverbound::Craft(*index)).unwrap();
            }
        }

        ui.add(Anchor::Center, view);
    }
}

pub fn add(world: World) -> World {
    let ui = CraftUi::new(&world, &data::recipes());
    world.with_system_mut(ui)
}
