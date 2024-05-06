#![feature(impl_trait_in_assoc_type)]
#![feature(vec_into_raw_parts)]

pub mod prelude;
pub mod scene;
pub mod utils;
mod vecany;

use serde::{
    de::{DeserializeSeed, Visitor},
    Deserialize, Serialize, Serializer,
};
use vecany::VecAny;

use std::{
    any::{Any, TypeId},
    cell::{Cell, Ref, RefCell, RefMut},
    collections::HashMap,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    rc::Rc,
};

pub trait System<E> {
    fn event(&self, _world: &World<E>, _event: &E) {}
    fn tick(&self, _world: &World<E>) {}
}

pub trait SystemMut<E> {
    fn tick(&mut self, _world: &World<E>) {}
}

struct Handler<T>(T);
struct Ticker<T>(T);

impl<E, T: Fn(&World<E>, &E)> System<E> for Handler<T> {
    fn event(&self, world: &World<E>, event: &E) {
        self.0(world, event)
    }
}

impl<E, T: Fn(&World<E>)> System<E> for Ticker<T> {
    fn tick(&self, world: &World<E>) {
        self.0(world)
    }
}

impl<E, T: SystemMut<E>> System<E> for RefCell<T> {
    fn tick(&self, world: &World<E>) {
        self.borrow_mut().tick(world)
    }
}

pub trait Archetype: Any {
    fn columns() -> Vec<TypeId>;
    fn add(self, table: &Table) -> RowIndex;
    fn remove(table: &Table, row: RowIndex);
    fn get(table: &Table, row: RowIndex) -> Self
    where
        Self: Clone;

    fn serialize(table: &Table, row: RowIndex) -> Box<dyn erased_serde::Serialize>
    where
        Self: Serialize + Clone,
    {
        Box::new(Self::get(table, row)) as Box<dyn erased_serde::Serialize>
    }
}

#[derive(Clone, Copy)]
pub(crate) struct DeserializeArchetype<'a> {
    table: &'a Table,
    func: fn(
        &Table,
        &mut dyn erased_serde::Deserializer<'_>,
    ) -> Result<RowIndex, erased_serde::Error>,
}

impl<'de> DeserializeSeed<'de> for DeserializeArchetype<'de> {
    type Value = RowIndex;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let mut deserializer = <dyn erased_serde::Deserializer>::erase(deserializer);
        Ok((self.func)(self.table, &mut deserializer).unwrap())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RowIndex(pub u32);
pub struct Column {
    pub data: VecAny,
}

impl Column {
    pub fn new(ty: TypeId) -> Self {
        let data = VecAny::new_uninit(ty);
        Self { data }
    }

    pub fn get<T: 'static>(&self, index: RowIndex) -> Option<&T> {
        self.data.downcast_ref()?.get(index.0 as usize)
    }

    pub fn get_mut<T: 'static>(&mut self, index: RowIndex) -> Option<&mut T> {
        self.data.downcast_mut()?.get_mut(index.0 as usize)
    }
}

pub struct Table {
    pub length: Cell<usize>,
    columns: Vec<(TypeId, RefCell<Column>)>,
    pub(crate) serialize: Option<fn(&Self, RowIndex) -> Box<dyn erased_serde::Serialize>>,
    pub(crate) deserialize: Option<
        fn(&Self, &mut dyn erased_serde::Deserializer<'_>) -> Result<RowIndex, erased_serde::Error>,
    >,
}

impl Table {
    pub fn new_unsaved<T: Archetype>() -> Self {
        Self {
            length: Cell::new(0),
            columns: T::columns()
                .iter()
                .cloned()
                .map(|ty| (ty, RefCell::new(Column::new(ty))))
                .collect(),
            serialize: None,
            deserialize: None,
        }
    }

    pub fn new<T: Archetype + Serialize + for<'a> Deserialize<'a> + Clone>() -> Self {
        Self {
            length: Cell::new(0),
            columns: T::columns()
                .iter()
                .cloned()
                .map(|ty| (ty, RefCell::new(Column::new(ty))))
                .collect(),
            serialize: Some(<T as Archetype>::serialize),
            deserialize: Some(
                |table: &Table, deserializer: &mut dyn erased_serde::Deserializer<'_>| {
                    <T as Deserialize>::deserialize(deserializer).map(|entity| entity.add(table))
                },
            ),
        }
    }

    pub fn columns_mut(&self) -> impl Iterator<Item = RefMut<'_, Column>> {
        self.columns.iter().map(|(_, column)| column.borrow_mut())
    }

    pub fn columns(&self) -> impl Iterator<Item = Ref<'_, Column>> {
        self.columns.iter().map(|(_, column)| column.borrow())
    }

    pub fn has_column<T: 'static>(&self) -> bool {
        self.columns.iter().any(|(ty, _)| *ty == TypeId::of::<T>())
    }

    pub fn column<T: 'static>(&self) -> Option<Ref<'_, [T]>> {
        self.columns
            .iter()
            .find(|(ty, _)| *ty == TypeId::of::<T>())
            .map(|(_, column)| {
                Ref::map(column.borrow(), |column| {
                    column.data.downcast_ref::<T>().unwrap_or_else(|| {
                        panic!("Failed to downcast {}", std::any::type_name::<T>())
                    })
                })
            })
    }

    pub fn column_mut<T: 'static>(&self) -> Option<RefMut<'_, [T]>> {
        self.columns
            .iter()
            .find(|(ty, _)| *ty == TypeId::of::<T>())
            .and_then(|(_, column)| {
                RefMut::filter_map(column.borrow_mut(), |column| {
                    column.data.downcast_mut::<T>()
                })
                .ok()
            })
    }

    pub fn len(&self) -> usize {
        self.length.get()
    }

    pub fn is_empty(&self) -> bool {
        self.length.get() == 0
    }
}

pub struct Columns<'a, T> {
    columns: Vec<Ref<'a, [T]>>,
}

impl<'a, T> FromIterator<Ref<'a, [T]>> for Columns<'a, T> {
    fn from_iter<I: IntoIterator<Item = Ref<'a, [T]>>>(iter: I) -> Self {
        Self {
            columns: iter.into_iter().collect(),
        }
    }
}

impl<'a, T> Columns<'a, T> {
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.columns.iter().flat_map(|column| column.deref())
    }
}

pub struct ColumnsMut<'a, T> {
    columns: Vec<RefMut<'a, [T]>>,
}

impl<'a, T> FromIterator<RefMut<'a, [T]>> for ColumnsMut<'a, T> {
    fn from_iter<I: IntoIterator<Item = RefMut<'a, [T]>>>(iter: I) -> Self {
        Self {
            columns: iter.into_iter().collect(),
        }
    }
}

impl<'a, T> ColumnsMut<'a, T> {
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.columns.iter().flat_map(|column| column.deref())
    }

    pub fn for_each<F: FnMut(&mut T)>(&mut self, f: F) {
        self.columns
            .iter_mut()
            .flat_map(|column| column.deref_mut())
            .for_each(f)
    }

    pub fn fold<A, F: FnMut(A, &mut T) -> A>(&mut self, init: A, f: F) -> A {
        self.columns
            .iter_mut()
            .flat_map(|column| column.deref_mut())
            .fold(init, f)
    }

    pub fn map<O, F: FnMut(&mut T) -> O>(&mut self, f: F) -> Vec<O> {
        self.columns
            .iter_mut()
            .flat_map(|column| column.deref_mut())
            .map(f)
            .collect()
    }

    pub fn filter_map<O, F: FnMut(&mut T) -> Option<O>>(&mut self, f: F) -> Vec<O> {
        self.columns
            .iter_mut()
            .flat_map(|column| column.deref_mut())
            .filter_map(f)
            .collect()
    }

    pub fn first(&mut self) -> Option<&mut T> {
        self.columns
            .first_mut()
            .and_then(|column| column.first_mut())
    }

    pub fn get_mut(&mut self, mut index: usize) -> Option<&mut T> {
        self.columns
            .iter_mut()
            .flat_map(|column| column.deref_mut())
            .find(|_| {
                if index == 0 {
                    true
                } else {
                    index -= 1;
                    false
                }
            })
    }
}

pub trait QueryOne<E> {
    type Output<'a>;

    fn filter(table: &(TypeId, &Table)) -> bool;
    fn data<'a>(tables: &[(TypeId, &'a Table)]) -> Self::Output<'a>;
}

impl<T: 'static, E> QueryOne<E> for &'_ T {
    type Output<'a> = Ref<'a, T>;

    fn filter(table: &(TypeId, &Table)) -> bool {
        table.1.has_column::<T>()
    }

    fn data<'a>(tables: &[(TypeId, &'a Table)]) -> Self::Output<'a> {
        tables
            .iter()
            .find_map(|(_, table)| {
                Ref::filter_map(table.column::<T>()?, |column| column.first()).ok()
            })
            .unwrap()
    }
}

impl<T: 'static, E> QueryOne<E> for &'_ mut T {
    type Output<'a> = RefMut<'a, T>;

    fn filter(table: &(TypeId, &Table)) -> bool {
        table.1.has_column::<T>()
    }

    fn data<'a>(tables: &[(TypeId, &'a Table)]) -> Self::Output<'a> {
        tables
            .iter()
            .find_map(|(_, table)| {
                RefMut::filter_map(table.column_mut::<T>()?, |column| column.first_mut()).ok()
            })
            .unwrap()
    }
}

macro_rules! impl_query_one {
    ($($ty:ident)+) => {
        impl<Event, $($ty: QueryOne<Event>),+> QueryOne<Event> for ($($ty),+,) {
            type Output<'a> = ($($ty::Output<'a>),+,);

            fn filter(table: &(TypeId, &Table)) -> bool {
                $($ty::filter(table))&&+
            }

            fn data<'a>(tables: &[(TypeId, &'a Table)]) -> Self::Output<'a> {
                ($($ty::data(tables)),+,)
            }
        }
    };
}

impl_query_one!(A);
impl_query_one!(A B);
impl_query_one!(A B C);
impl_query_one!(A B C D);
impl_query_one!(A B C D E);
impl_query_one!(A B C D E F);
impl_query_one!(A B C D E F G);
impl_query_one!(A B C D E F G H);

pub trait Query<E> {
    type Output<'a>;

    fn filter(table: &(TypeId, &Table)) -> bool;
    fn data<'a>(
        entities: &HashMap<EntityId, (TypeId, RowIndex)>,
        tables: &[(TypeId, &'a Table)],
    ) -> Self::Output<'a>;
}

impl<T: 'static, E> Query<E> for &'_ T {
    type Output<'a> = Columns<'a, T>;

    fn filter(table: &(TypeId, &Table)) -> bool {
        table.1.has_column::<T>()
    }

    fn data<'a>(
        _: &HashMap<EntityId, (TypeId, RowIndex)>,
        tables: &[(TypeId, &'a Table)],
    ) -> Self::Output<'a> {
        tables
            .iter()
            .map(|(_, table)| table.column().unwrap())
            .collect()
    }
}

impl<T: 'static, E> Query<E> for &'_ mut T {
    type Output<'a> = ColumnsMut<'a, T>;

    fn filter(table: &(TypeId, &Table)) -> bool {
        table.1.has_column::<T>()
    }

    fn data<'a>(
        _: &HashMap<EntityId, (TypeId, RowIndex)>,
        tables: &[(TypeId, &'a Table)],
    ) -> Self::Output<'a> {
        tables
            .iter()
            .map(|(_, table)| table.column_mut().unwrap())
            .collect()
    }
}

macro_rules! impl_query {
    ($($ty:ident)+) => {
        impl<Event, $($ty: Query<Event>),+> Query<Event> for ($($ty),+,) {
            type Output<'a> = ($($ty::Output<'a>),+,);

            fn filter(table: &(TypeId, &Table)) -> bool {
                $($ty::filter(table))&&+
            }

            fn data<'a>(entities: &HashMap<EntityId, (TypeId, RowIndex)>, tables: &[(TypeId, &'a Table)]) -> Self::Output<'a> {
                ($($ty::data(entities, tables)),+,)
            }
        }
    };
}

impl_query!(A);
impl_query!(A B);
impl_query!(A B C);
impl_query!(A B C D);
impl_query!(A B C D E);
impl_query!(A B C D E F);
impl_query!(A B C D E F G);
impl_query!(A B C D E F G H);

pub struct With<T>(PhantomData<T>);
impl<E, T: 'static> Query<E> for With<T> {
    type Output<'a> = ();

    fn filter(table: &(TypeId, &Table)) -> bool {
        table.1.has_column::<T>()
    }

    fn data<'a>(
        _: &HashMap<EntityId, (TypeId, RowIndex)>,
        _: &[(TypeId, &'a Table)],
    ) -> Self::Output<'a> {
    }
}
impl<E, T: 'static> QueryOne<E> for With<T> {
    type Output<'a> = ();

    fn filter(table: &(TypeId, &Table)) -> bool {
        table.1.has_column::<T>()
    }

    fn data<'a>(_: &[(TypeId, &'a Table)]) -> Self::Output<'a> {}
}

pub struct Without<T>(PhantomData<T>);
impl<E, T: 'static> Query<E> for Without<T> {
    type Output<'a> = ();

    fn filter(table: &(TypeId, &Table)) -> bool {
        !table.1.has_column::<T>()
    }

    fn data<'a>(
        _: &HashMap<EntityId, (TypeId, RowIndex)>,
        _: &[(TypeId, &'a Table)],
    ) -> Self::Output<'a> {
    }
}
impl<E, T: 'static> QueryOne<E> for Without<T> {
    type Output<'a> = ();

    fn filter(table: &(TypeId, &Table)) -> bool {
        !table.1.has_column::<T>()
    }

    fn data<'a>(_: &[(TypeId, &'a Table)]) -> Self::Output<'a> {}
}

pub struct Is<T>(PhantomData<T>);
impl<E, T: Archetype> Query<E> for Is<T> {
    type Output<'a> = ();

    fn filter(table: &(TypeId, &Table)) -> bool {
        table.0 == TypeId::of::<T>()
    }

    fn data<'a>(
        _: &HashMap<EntityId, (TypeId, RowIndex)>,
        _: &[(TypeId, &'a Table)],
    ) -> Self::Output<'a> {
    }
}
impl<E, T: Archetype> QueryOne<E> for Is<T> {
    type Output<'a> = ();

    fn filter(table: &(TypeId, &Table)) -> bool {
        table.0 == TypeId::of::<T>()
    }

    fn data<'a>(_: &[(TypeId, &'a Table)]) -> Self::Output<'a> {}
}

pub struct ColumnsOptional<'a, T> {
    columns: Vec<Result<Ref<'a, [T]>, usize>>,
}

impl<'a, T> FromIterator<Result<Ref<'a, [T]>, usize>> for ColumnsOptional<'a, T> {
    fn from_iter<I: IntoIterator<Item = Result<Ref<'a, [T]>, usize>>>(iter: I) -> Self {
        Self {
            columns: iter.into_iter().collect(),
        }
    }
}

impl<'a, T> ColumnsOptional<'a, T> {
    pub fn iter(&self) -> impl Iterator<Item = Option<&T>> {
        self.columns.iter().flat_map(|column| match column {
            Ok(column) => column.iter().map(Some).collect::<Vec<Option<&T>>>(),
            Err(size) => vec![None; *size],
        })
    }
}

impl<E, T: Archetype> Query<E> for Option<&'_ T> {
    type Output<'a> = ColumnsOptional<'a, T>;

    fn filter(_: &(TypeId, &Table)) -> bool {
        true
    }

    fn data<'a>(
        _: &HashMap<EntityId, (TypeId, RowIndex)>,
        tables: &[(TypeId, &'a Table)],
    ) -> Self::Output<'a> {
        tables
            .iter()
            .map(|(_, table)| table.column::<T>().ok_or_else(|| table.len()))
            .collect()
    }
}

impl<E> Query<E> for EntityId {
    type Output<'a> = Vec<EntityId>;

    fn filter(_: &(TypeId, &Table)) -> bool {
        true
    }

    fn data<'a>(
        entities: &HashMap<EntityId, (TypeId, RowIndex)>,
        tables: &[(TypeId, &'a Table)],
    ) -> Self::Output<'a> {
        tables
            .iter()
            .flat_map(|(ty, table)| {
                (0..table.len()).filter_map(|i| {
                    entities
                        .iter()
                        .find(|(_, (t, row))| *t == *ty && i == row.0 as usize)
                })
            })
            .map(|(id, _)| *id)
            .collect()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct EntityId(u64);

pub struct World<E> {
    next_id: Cell<u64>,
    entities: RefCell<HashMap<EntityId, (TypeId, RowIndex)>>,
    archetypes: HashMap<TypeId, Table>,
    systems: Vec<Rc<dyn System<E>>>,
    resources: HashMap<TypeId, Rc<RefCell<dyn Any>>>,
}

impl<E> Default for World<E> {
    fn default() -> Self {
        Self {
            next_id: Cell::new(0),
            entities: RefCell::new(HashMap::new()),
            archetypes: HashMap::new(),
            systems: Vec::new(),
            resources: HashMap::new(),
        }
    }
}

impl<E> World<E> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with<F: FnOnce(Self) -> Self>(self, f: F) -> Self {
        f(self)
    }

    pub fn with_system<T: System<E> + 'static>(mut self, system: T) -> Self {
        self.systems.push(Rc::new(system));
        self
    }

    pub fn with_system_mut<T: SystemMut<E> + 'static>(mut self, system: T) -> Self {
        self.systems.push(Rc::new(RefCell::new(system)));
        self
    }

    pub fn with_handler<T: Fn(&World<E>, &E) + 'static>(mut self, handler: T) -> Self {
        self.systems.push(Rc::new(Handler(handler)));
        self
    }

    pub fn with_ticker<T: Fn(&World<E>) + 'static>(mut self, ticker: T) -> Self {
        self.systems.push(Rc::new(Ticker(ticker)));
        self
    }

    pub fn with_resource<T: Any>(mut self, resource: T) -> Self {
        self.resources
            .insert(TypeId::of::<T>(), Rc::new(RefCell::new(resource)));
        self
    }

    pub fn register<T: Archetype + Serialize + for<'a> Deserialize<'a> + Clone>(mut self) -> Self {
        self.archetypes.insert(TypeId::of::<T>(), Table::new::<T>());
        self
    }

    pub fn register_unsaved<T: Archetype>(mut self) -> Self {
        self.archetypes
            .insert(TypeId::of::<T>(), Table::new_unsaved::<T>());
        self
    }

    pub fn spawn<T: Archetype>(&self, entity: T) -> EntityId {
        if !self.archetypes.contains_key(&TypeId::of::<T>()) {
            panic!("Unregistered archetype {}", std::any::type_name::<T>());
        }

        let store = self.archetypes.get(&TypeId::of::<T>()).unwrap();
        entity.add(store);
        self.entities.borrow_mut().insert(
            EntityId(self.next_id.get()),
            (TypeId::of::<T>(), RowIndex(store.len() as u32 - 1)),
        );
        self.next_id.set(self.next_id.get() + 1);
        EntityId(self.next_id.get() - 1)
    }

    pub fn despawn<T: Archetype + 'static>(&self, entity: EntityId) {
        let mut entities = self.entities.borrow_mut();
        let Some((table_id, row)) = entities.remove(&entity) else {
            return;
        };

        if table_id != TypeId::of::<T>() {
            panic!("Despawn archetype mismatch")
        }

        let Some(table) = self.archetypes.get(&table_id) else {
            return;
        };
        T::remove(table, row);
        entities
            .values_mut()
            .find(|(t, r)| t == &table_id && r.0 == table.len() as u32)
            .map(|(_, r)| *r = row);
    }

    pub fn query<Q: Query<E>>(&self) -> Q::Output<'_> {
        Q::data(
            &self.entities.borrow(),
            &self
                .archetypes
                .iter()
                .map(|(&ty, table)| (ty, table))
                .filter(Q::filter)
                .collect::<Vec<_>>(),
        )
    }

    pub fn query_one<Q: QueryOne<E>>(&self) -> Q::Output<'_> {
        Q::data(
            &self
                .archetypes
                .iter()
                .map(|(&ty, table)| (ty, table))
                .filter(Q::filter)
                .collect::<Vec<_>>(),
        )
    }

    pub fn get_component<T: 'static>(&self, id: EntityId) -> Option<Ref<'_, T>> {
        let (table, row) = self.entities.borrow().get(&id).copied()?;
        let table = self
            .archetypes
            .get(&table)
            .expect("Using unregistered archetype");
        Ref::filter_map(table.column::<T>()?, |column| column.get(row.0 as usize)).ok()
    }

    pub fn get_component_mut<T: 'static>(&self, id: EntityId) -> Option<RefMut<'_, T>> {
        let (table, row) = self.entities.borrow().get(&id).copied()?;
        let table = self
            .archetypes
            .get(&table)
            .expect("Using unregistered archetype");
        RefMut::filter_map(table.column_mut::<T>()?, |column| {
            column.get_mut(row.0 as usize)
        })
        .ok()
    }

    pub fn get<T: Any>(&self) -> Option<Ref<'_, T>> {
        self.resources
            .get(&TypeId::of::<T>())
            .map(|resource| Ref::map(resource.borrow(), |x| x.downcast_ref().unwrap()))
    }

    pub fn get_mut<T: Any>(&self) -> Option<RefMut<'_, T>> {
        self.resources
            .get(&TypeId::of::<T>())
            .map(|resource| RefMut::map(resource.borrow_mut(), |x| x.downcast_mut().unwrap()))
    }

    pub fn remove<T: Any>(&mut self) -> Option<T> {
        self.resources.remove(&TypeId::of::<T>()).and_then(|rc| {
            let ptr: *const RefCell<dyn Any> = Rc::into_raw(rc);
            let ptr: *const RefCell<T> = ptr.cast();
            unsafe { Rc::into_inner(Rc::from_raw(ptr)).map(|x| x.into_inner()) }
        })
    }

    pub fn tick(&self) {
        self.systems
            .clone()
            .into_iter()
            .for_each(|system| system.tick(self))
    }

    pub fn submit(&self, event: E) {
        self.systems
            .clone()
            .into_iter()
            .for_each(|system| system.event(self, &event))
    }
}
