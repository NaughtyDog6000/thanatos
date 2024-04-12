use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Item {
    Wood,
    CopperOre,
    CopperIngot,
    CopperSword
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ItemStack {
    pub item: Item,
    pub quantity: usize
}

#[derive(Default, Debug)]
pub struct Inventory(HashMap<Item, usize>);

impl Inventory {
    pub fn add(&mut self, stack: ItemStack) {
        match self.0.get_mut(&stack.item) {
            Some(quantity) => *quantity += stack.quantity,
            None => { self.0.insert(stack.item, stack.quantity); }
        }
    } 

    pub fn get(&self, item: Item) -> Option<usize> {
        self.0.get(&item).copied()
    }
}
