use glam::Vec4;
use styx::{
    components::{text, Clicked, Container, HAlign, HGroup, Text, VAlign, VGroup},
    Signal,
};
use tecs::{System, SystemMut};

use crate::{
    event::Event,
    item::{Inventory, Item, ItemStack},
    renderer::{Anchor, Ui},
    window::Keyboard,
    World,
};

#[derive(Clone, Debug, Default)]
pub struct Recipe {
    inputs: Vec<ItemStack>,
    outputs: Vec<ItemStack>,
}

impl Recipe {
    pub fn craftable(&self, inventory: &Inventory) -> bool {
        self.inputs
            .iter()
            .all(|stack| stack.quantity <= inventory.get(stack.item).unwrap_or_default())
    }
}

pub struct CraftUi {
    open: bool,
    craft: Signal,
    recipe: Option<Recipe>,
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

        let recipes = self.recipes.iter().fold(
            HGroup::new(HAlign::Left, 16.0),
            |component, (signal, recipe)| {
                let output = recipe.outputs.first().unwrap();

                if ui.signals.get(*signal) {
                    self.recipe = Some(recipe.clone())
                }

                let colour = if recipe.craftable(&inventory) {
                    Vec4::new(0.0, 1.0, 0.0, 1.0)
                } else {
                    Vec4::new(1.0, 0.0, 0.0, 1.0)
                };

                component.add(Clicked {
                    signal: *signal,
                    child: Text {
                        text: format!("{} x {}", output.item, output.quantity),
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

        if let Some(recipe) = &self.recipe {
            let inputs = recipe.inputs.iter().fold(
                HGroup::new(HAlign::Left, 16.0).add(text("Inputs:", 48.0, ui.font.clone())),
                |inputs, input| {
                    let colour = if input.quantity <= inventory.get(input.item).unwrap_or_default()
                    {
                        Vec4::new(0.0, 1.0, 0.0, 1.0)
                    } else {
                        Vec4::new(1.0, 0.0, 0.0, 1.0)
                    };

                    inputs.add(Text {
                        text: format!("{} x {}", input.item, input.quantity),
                        font_size: 48.0,
                        font: ui.font.clone(),
                        colour,
                    })
                },
            );

            let outputs = recipe.outputs.iter().fold(
                HGroup::new(HAlign::Left, 16.0).add(text("Outputs:", 48.0, ui.font.clone())),
                |outputs, output| {
                    outputs.add(Text {
                        text: format!("{} x {}", output.item, output.quantity),
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

        if let Some(recipe) = &self.recipe {
            if recipe.craftable(&inventory) && ui.signals.get(self.craft) {
                recipe
                    .inputs
                    .iter()
                    .for_each(|stack| inventory.remove(*stack).unwrap());
                recipe
                    .outputs
                    .iter()
                    .for_each(|stack| inventory.add(*stack));
            }
        }

        ui.add(Anchor::Center, view);
    }
}

pub fn add(world: World) -> World {
    let ui = CraftUi::new(
        &world,
        &[
            Recipe {
                inputs: vec![ItemStack {
                    item: Item::CopperOre,
                    quantity: 2,
                }],
                outputs: vec![ItemStack {
                    item: Item::CopperIngot,
                    quantity: 1,
                }],
            },
            Recipe {
                inputs: vec![ItemStack {
                    item: Item::CopperIngot,
                    quantity: 3,
                }],
                outputs: vec![ItemStack {
                    item: Item::CopperSword,
                    quantity: 1,
                }],
            },
        ],
    );
    world.with_system_mut(ui)
}
