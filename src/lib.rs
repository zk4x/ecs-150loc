//! Very simple no-std ECS.
//! Entities are just u32, componenents can be all types that implement Default + 'static
//! This ECS is meant to be used with data where most components are shared by all entities (dense data).
//! If this is not the case (sparse data), create multiple data structs.
//! Compared to using raw `Vec<T>` there are two overheads:
//! 1. query makes single dynamic function call (i. e. one vtable lookup)
//! 2. data contains reference count for each entity, but it is purely manual and only 1 byte per entity, thus max is 255 references of one entity

#![no_std]
#![deny(clippy::pedantic)]

extern crate alloc;
use core::any::TypeId;

/// Entity
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub struct Entity(u32);

/// Data
///
/// ```
/// use ecs::Data;
///
/// #[derive(Default)]
/// struct Position(u64, u64);
///
/// #[derive(Default)]
/// struct Velocity(f64, f64);
///
/// let mut world = Data::new();
/// let player = world.entity();
/// world.insert(player, Position(10, 20));
/// world.insert(player, Velocity(10., 20.));
///
/// let player2 = world.entity();
/// world.insert(player2, Position(10, 20));
///
/// world.query_mut::<Position>().unwrap()[player.i()].1 += 1;
/// ```
#[derive(Default)]
pub struct Data {
    // This can be either reference count or generational index, depending on usecase
    rc: alloc::vec::Vec<u8>,
    // we can have both dense components with vec
    // and sparse componenets with BTreeMap<Entity, impl Component>
    components: alloc::collections::BTreeMap<TypeId, alloc::boxed::Box<dyn Storage>>,
}

impl Entity {
    /// Get self as usize.
    /// # Panics
    /// Panics if u32 can not be converted into usize.
    #[must_use]
    pub fn i(self) -> usize {
        self.0.try_into().unwrap()
    }
}

trait Storage: 'static {
    fn push_item(&mut self);
}

impl<T: Default + 'static> Storage for alloc::vec::Vec<T> {
    fn push_item(&mut self) {
        self.push(Default::default());
    }
}

trait Downcast {
    fn downcast_ref<T: 'static>(&self) -> &T;
    fn downcast_mut<T: 'static>(&mut self) -> &mut T;
}

impl Downcast for alloc::boxed::Box<dyn Storage> {
    fn downcast_ref<T: 'static>(&self) -> &T {
        unsafe { &*(self.as_ref() as *const dyn Storage).cast() }
    }

    fn downcast_mut<T: 'static>(&mut self) -> &mut T {
        unsafe { &mut *(self.as_mut() as *mut dyn Storage).cast() }
    }
}

impl Data {
    /// Initialize empty new system
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add new entity to the system
    #[allow(clippy::missing_panics_doc)]
    #[must_use]
    pub fn entity(&mut self) -> Entity {
        let id = self.rc.len();
        self.rc.push(1);
        for component in self.components.values_mut() {
            component.push_item();
        }
        Entity(u32::try_from(id).unwrap())
    }

    /// Add component to existing entity
    #[allow(clippy::missing_panics_doc)]
    pub fn insert<T: Default + 'static>(&mut self, entity: Entity, component: T) -> bool {
        if let alloc::collections::btree_map::Entry::Vacant(e) =
            self.components.entry(TypeId::of::<T>())
        {
            e.insert(alloc::boxed::Box::new(alloc::vec![component]));
            false
        } else {
            self.query_mut::<T>().unwrap()[entity.i()] = component;
            true
        }
    }

    /// Query all values of single component type
    #[must_use]
    pub fn query<T: 'static>(&self) -> Option<&[T]> {
        self.components
            .get(&TypeId::of::<T>())
            .map(|x| x.downcast_ref::<alloc::vec::Vec<T>>().as_ref())
    }

    /// Mutably query all values of single component type
    #[must_use]
    pub fn query_mut<T: 'static>(&mut self) -> Option<&mut [T]> {
        self.components
            .get_mut(&TypeId::of::<T>())
            .map(|x| x.downcast_mut::<alloc::vec::Vec<T>>().as_mut())
    }

    /// Increase reference count of single entity
    pub fn retain(&mut self, entity: Entity) {
        self.rc[entity.i()] += 1;
    }

    /// Decrease reference count of single entity
    pub fn release(&mut self, entity: Entity) {
        self.rc[entity.i()] -= 1;
    }
}
