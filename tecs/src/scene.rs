use std::{
    any::TypeId,
    collections::HashMap,
    hash::{DefaultHasher, Hash, Hasher},
};

use erased_serde::Serialize;

use crate::{EntityId, RowIndex, World};

#[derive(Clone, Default)]
pub struct Scene {
    entities: Vec<EntityId>,
}

impl Scene {
    pub fn add(&mut self, entity: EntityId) {
        self.entities.push(entity)
    }

    pub fn from_world<E>(&mut self, world: &World<E>) {
        world.entities.borrow().keys().copied().for_each(|id| self.add(id));
    }

    pub fn save<E, S: serde::Serializer>(
        &self,
        world: &World<E>,
        serializer: S,
    ) -> Result<(), erased_serde::Error> {
        let mut serializer = <dyn erased_serde::Serializer>::erase(serializer);
        let entities = world.entities.borrow();
        let mut entity_map: HashMap<TypeId, Vec<RowIndex>> = HashMap::new();

        self.entities
            .iter()
            .filter_map(|id| entities.get(&id).cloned())
            .for_each(|(table, row)| match entity_map.get_mut(&table) {
                Some(rows) => rows.push(row),
                None => {
                    entity_map.insert(table, vec![row]);
                }
            });

        let scene: Vec<(u64, Vec<Box<dyn Serialize>>)> = entity_map
            .into_iter()
            .map(|(id, rows)| {
                let table = world.archetypes.get(&id).unwrap();
                let rows = rows
                    .into_iter()
                    .map(|row| (table.serialize.unwrap())(table, row))
                    .collect::<Vec<_>>();

                let mut hasher = DefaultHasher::new();
                id.hash(&mut hasher);
                (hasher.finish(), rows)
            })
            .collect::<Vec<_>>();

        (Box::new(scene) as Box<dyn erased_serde::Serialize>).erased_serialize(&mut serializer)?;

        Ok(())
    }
}
