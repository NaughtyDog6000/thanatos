use std::{
    any::TypeId,
    collections::HashMap,
    hash::{DefaultHasher, Hash, Hasher},
};

use erased_serde::Serialize;
use serde::{de::{DeserializeSeed, Visitor}, Deserializer};

use crate::{DeserializeArchetype, EntityId, RowIndex, Table, World};

#[derive(Clone, Default)]
pub struct Scene {
    entities: Vec<EntityId>,
}

impl Scene {
    pub fn add(&mut self, entity: EntityId) {
        self.entities.push(entity)
    }

    pub fn from_world<E>(&mut self, world: &World<E>) {
        world
            .entities
            .borrow()
            .keys()
            .copied()
            .for_each(|id| self.add(id));
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

        let scene: HashMap<u64, Vec<Box<dyn Serialize>>> = entity_map
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
            .collect();

        (Box::new(scene) as Box<dyn erased_serde::Serialize>).erased_serialize(&mut serializer)?;

        Ok(())
    }

    pub fn load<'a, E, D: serde::Deserializer<'a>>(
        world: &'a World<E>,
        deserializer: D,
    ) -> Result<Self, <D as Deserializer>::Error> {
        deserializer.deserialize_map(ArchetypesSeed { world })
    }
}

#[derive(Clone, Copy)]
struct EntitiesSeed<'a> {
    table: &'a Table,
}

impl<'de> DeserializeSeed<'de> for EntitiesSeed<'de> {
    type Value = Vec<RowIndex>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_seq(self)
    }
}

impl<'de> Visitor<'de> for EntitiesSeed<'de> {
    type Value = Vec<RowIndex>;

    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Expect entities")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let mut rows = Vec::new();
        let seed = DeserializeArchetype {
            table: self.table,
            func: self.table.deserialize.unwrap(),
        };
        while let Some(row) = seq.next_element_seed(seed)? {
            rows.push(row);
        }
        Ok(rows)
    }
}

struct ArchetypesSeed<'a, E> {
    world: &'a World<E>,
}

impl<'de, E> Visitor<'de> for ArchetypesSeed<'de, E> {
    type Value = Scene;

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        let mut entities: Vec<EntityId> = Vec::new();
        while let Some(hash) = map.next_key()? {
            let (id, table) = self
                .world
                .archetypes
                .iter()
                .find(|(id, _)| {
                    let mut hasher = DefaultHasher::new();
                    id.hash(&mut hasher);
                    let id_hash = hasher.finish();
                    id_hash == hash
                })
                .expect("Missing archetype needed to deserialize scene");

            let seed = EntitiesSeed { table };
            let mut world = self.world.entities.borrow_mut();
            let rows = map.next_value_seed(seed)?;
            rows.into_iter().for_each(|row| {
                world.insert(EntityId(self.world.next_id.get()), (*id, row));
                entities.push(EntityId(self.world.next_id.get()));
                self.world.next_id.set(self.world.next_id.get() + 1)
            })
        }

        Ok(Scene {
            entities: Vec::new(),
        })
    }

    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Map of archetype hashes to entities")
    }
}
