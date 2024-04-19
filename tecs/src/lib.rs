#![feature(impl_trait_in_assoc_type)]
#![feature(vec_into_raw_parts)]

mod vecany;
pub mod utils;

pub use vecany::VecAny;

use std::{
    any::{Any, TypeId},
    cell::{Cell, Ref, RefCell, RefMut},
    collections::HashMap,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    rc::Rc,
};

pub trait System<E> {
    fn event(&self, world: &World<E>, event: &E) {}
    fn tick(&self, world: &World<E>) {}
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

pub trait Archetype: Any {
    fn columns() -> Vec<TypeId>;
    fn add(self, table: &Table);
}

#[macro_export]
macro_rules! impl_archetype {
    ($(pub )?struct $for:ident { $( pub $field:ident: $type:ty ),* $(,)?}) => {
        /*
        concat_idents::concat_idents!(for_ref = $for, Ref {
            pub struct for_ref<'a> {
                $($field: &'a $type,)*
            }
        });

        concat_idents::concat_idents!(for_mut = $for, Mut {
            pub struct for_mut<'a> {
                $($field: &'a mut $type,)*
            }
        });
*/

        impl tecs::Archetype for $for {
            /*
            concat_idents::concat_idents!(for_ref = $for, Ref {
                type Ref<'a> = for_ref<'a>;

                fn from_components<'a>(components: &'a std::collections::HashMap<std::any::TypeId, std::cell::RefCell<tecs::VecAny>>, indices: &[u32]) -> for_ref<'a> {
                    let mut indices = indices.iter();

                    for_ref {
                        $($field: std::cell::Ref::map(components.get(&std::any::TypeId::of::<$type>()).unwrap().borrow(), |x| x.downcast_ref::<$type>().unwrap().get(*indices.next().unwrap() as usize).unwrap()),)*
                    }
                }
            });
            concat_idents::concat_idents!(for_mut = $for, Mut {
                type Mut<'a> = for_mut<'a>;

                fn from_components_mut<'a>(components: &'a std::collections::HashMap<std::any::TypeId, std::cell::RefCell<tecs::VecAny>>, indices: &[u32]) -> for_mut<'a> {
                    let mut indices = indices.iter();

                    for_mut {
                        $($field: std::cell::RefMut::map(components.get(&std::any::TypeId::of::<$type>()).unwrap().borrow_mut(), |x| x.downcast_mut::<$type>().unwrap().get_mut(*indices.next().unwrap() as usize).unwrap()),)*
                    }
                }
            });
            */

            fn columns() -> Vec<std::any::TypeId> {
                vec![$(std::any::TypeId::of::<$type>()),*]
            }

            fn add(self, table: &tecs::Table) {
                table.length.set(table.length.get() + 1);
                let mut columns = table.columns_mut();
                $(
                    columns.next().unwrap().push::<$type>(self.$field);
                )*
            }


        }
    };
}

pub struct RowIndex(u32);
pub struct Column {
    data: VecAny,
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

    pub fn push<T: 'static>(&mut self, item: T) {
        self.data.push(item)
    }
}

pub struct Table {
    pub length: Cell<usize>,
    columns: Vec<(TypeId, RefCell<Column>)>,
}

impl Table {
    pub fn new(columns: &[TypeId]) -> Self {
        Self {
            length: Cell::new(0),
            columns: columns
                .iter()
                .cloned()
                .map(|ty| (ty, RefCell::new(Column::new(ty))))
                .collect(),
        }
    }

    pub fn columns_mut(&self) -> impl Iterator<Item = RefMut<'_, Column>> {
        self.columns.iter().map(|(_, column)| column.borrow_mut())
    }

    pub fn has_column<T: 'static>(&self) -> bool {
        self.columns
            .iter()
            .find(|(ty, _)| *ty == TypeId::of::<T>())
            .is_some()
    }

    pub fn column<T: 'static>(&self) -> Option<Ref<'_, [T]>> {
        self.columns
            .iter()
            .find(|(ty, _)| *ty == TypeId::of::<T>())
            .map(|(_, column)| {
                Ref::map(column.borrow(), |column| {
                    column.data.downcast_ref::<T>().expect(&format!(
                        "Failed to downcast {}",
                        std::any::type_name::<T>()
                    ))
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
    fn data<'a>(tables: &[(TypeId, &'a Table)]) -> Self::Output<'a>;
}

impl<T: 'static, E> Query<E> for &'_ T {
    type Output<'a> = Columns<'a, T>;

    fn filter(table: &(TypeId, &Table)) -> bool {
        table.1.has_column::<T>()
    }

    fn data<'a>(tables: &[(TypeId, &'a Table)]) -> Self::Output<'a> {
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

    fn data<'a>(tables: &[(TypeId, &'a Table)]) -> Self::Output<'a> {
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

            fn data<'a>(tables: &[(TypeId, &'a Table)]) -> Self::Output<'a> {
                ($($ty::data(tables)),+,)
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

    fn data<'a>(_: &[(TypeId, &'a Table)]) -> Self::Output<'a> {}
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

    fn data<'a>(_: &[(TypeId, &'a Table)]) -> Self::Output<'a> {}
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

    fn data<'a>(_: &[(TypeId, &'a Table)]) -> Self::Output<'a> {}
}
impl<E, T: Archetype> QueryOne<E> for Is<T> {
    type Output<'a> = ();

    fn filter(table: &(TypeId, &Table)) -> bool {
        table.0 == TypeId::of::<T>()
    }

    fn data<'a>(_: &[(TypeId, &'a Table)]) -> Self::Output<'a> {}
}

impl<E> Query<E> for EntityId {
    type Output<'a> = Vec<EntityId>;

    fn filter(_: &(TypeId, &Table)) -> bool {
        true
    }

    fn data<'a>(tables: &[(TypeId, &'a Table)]) -> Self::Output<'a> {
        tables
            .iter()
            .flat_map(|(ty, table)| (0..table.len()).map(|i| EntityId(i as u32, *ty)))
            .collect()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TypedEntityId<T>(u32, PhantomData<T>);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct EntityId(u32, TypeId);

impl<T: 'static> From<TypedEntityId<T>> for EntityId {
    fn from(value: TypedEntityId<T>) -> Self {
        Self(value.0, TypeId::of::<T>())
    }
}

pub struct World<E> {
    archetypes: HashMap<TypeId, Table>,
    systems: Vec<Rc<dyn System<E>>>,
    resources: HashMap<TypeId, Rc<RefCell<dyn Any>>>,
}

impl<E> Default for World<E> {
    fn default() -> Self {
        Self {
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

    pub fn register<T: Archetype>(mut self) -> Self {
        self.archetypes
            .insert(TypeId::of::<T>(), Table::new(&T::columns()));
        self
    }

    pub fn spawn<T: Archetype>(&self, entity: T) -> TypedEntityId<T> {
        if !self.archetypes.contains_key(&TypeId::of::<T>()) {
            panic!("Unregistered archetype {}", std::any::type_name::<T>());
        }

        let store = self.archetypes.get(&TypeId::of::<T>()).unwrap();
        entity.add(store);
        TypedEntityId(store.len() as u32 - 1, PhantomData)
    }

    pub fn query<Q: Query<E>>(&self) -> Q::Output<'_> {
        Q::data(
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
        let table = self
            .archetypes
            .get(&id.1)
            .expect("Using unregistered archetype");
        Ref::filter_map(table.column::<T>()?, |column| column.get(id.0 as usize)).ok()
    }

    pub fn get_component_mut<T: 'static>(&self, id: EntityId) -> Option<RefMut<'_, T>> {
        let table = self
            .archetypes
            .get(&id.1)
            .expect("Using unregistered archetype");
        RefMut::filter_map(table.column_mut::<T>()?, |column| {
            column.get_mut(id.0 as usize)
        })
        .ok()
    }

    /*
    pub fn get_entities<T: Archetype>(&self) -> impl Iterator<Item = &T> {
        let Some(table) = self.archetypes.get(&TypeId::of::<T>()) else {
            return &[];
        };

        (0..table.borrow().len()).map(|index| T::from_components(components, indices))
    }

    pub fn get_entities_mut<T: Archetype>(&self) -> Vec<T::Mut<'_>> {
        let rows = &self.archetypes.get(&TypeId::of::<T>()).unwrap().rows;

        let mut output = Vec::new();
        for indices in rows {
            output.push(T::from_components_mut(&self.components, indices));
        }

        output
    }
    */

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
