//! Requirements
//!
//! 1. Reactive components must be typed
//! 2. Reactive update must be synchronous and run to completion
//!
//! Can side effects be synchronous if the update is run in an exclusive system?

use std::{collections::HashSet, marker::PhantomData};

use bevy_ecs::prelude::*;
use derived::{write_observable, Derivable, Derived};

pub mod derived;

/// An observable component that can only be accessed through the [`ReactiveContext`].
#[derive(Component)]
pub struct Obs<T: Send + Sync + PartialEq + 'static> {
    rctx_entity: Entity,
    p: PhantomData<T>,
}

impl<T: Send + Sync + PartialEq + 'static> Clone for Obs<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: Send + Sync + PartialEq + 'static> Copy for Obs<T> {}

impl<T: Send + Sync + PartialEq + 'static> Obs<T> {
    pub fn read<'r>(&self, rctx: &'r mut ReactiveContext) -> &'r T {
        &rctx
            .world
            .get::<Observable<T>>(self.rctx_entity)
            .unwrap()
            .data
    }

    /// Potentially expensive operation that will write a value to this observable, or "signal".
    /// This will cause all reactive subscribers of this observable to recompute their own values,
    /// which can cause all of its subscribers to recompute, etc.
    pub fn write_and_react(&self, rctx: &mut ReactiveContext, value: T) {
        write_observable(&mut rctx.world, self.rctx_entity, value)
    }
}

/// A derived component whose value is computed automatically, and can only be read through the
/// [`ReactiveContext`].
#[derive(Component)]
pub struct Der<T: Send + Sync + 'static> {
    rctx_entity: Entity,
    p: PhantomData<T>,
}

impl<T: Send + Sync + 'static> Clone for Der<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: Send + Sync + 'static> Copy for Der<T> {}

impl<T: Send + Sync + 'static> Der<T> {
    pub fn read<'r>(&self, rctx: &'r mut ReactiveContext) -> &'r T {
        &rctx
            .world
            .get::<Observable<T>>(self.rctx_entity)
            .unwrap()
            .data
    }
}

#[derive(Default)]
pub struct ReactiveContext {
    world: World,
}

impl ReactiveContext {
    /// Returns a reference to the current value of the provided observable.
    pub fn read<T: Send + Sync + PartialEq + 'static>(&mut self, observable: Obs<T>) -> &T {
        // get the obs data from the world
        // add the reader to the obs data's subs
        &self
            .world
            .get::<Observable<T>>(observable.rctx_entity)
            .unwrap()
            .data
    }

    /// Potentially expensive operation that will write a value to this observable, or "signal".
    /// This will cause all reactive subscribers of this observable to recompute their own values,
    /// which can cause all of its subscribers to recompute, etc.
    pub fn write_and_react<T: Send + Sync + PartialEq + 'static>(
        &mut self,
        observable: Obs<T>,
        value: T,
    ) {
        observable.write_and_react(self, value);
    }

    pub fn add_observable<T: Send + Sync + PartialEq + 'static>(
        &mut self,
        initial_value: T,
    ) -> Obs<T> {
        let rctx_entity = self
            .world
            .spawn(Observable {
                data: initial_value,
                subscribers: Default::default(),
            })
            .id();
        Obs {
            rctx_entity,
            p: PhantomData,
        }
    }

    pub fn add_derived<C: Send + Sync + PartialEq + 'static, D: Derivable<C> + 'static>(
        &mut self,
        input_deps: D,
        derive_fn: fn(D::Query<'_>) -> C,
    ) -> Der<C> {
        let entity = self.world.spawn_empty().id();
        let mut derived = Derived::new(entity, input_deps, derive_fn);
        derived.execute(&mut self.world);
        self.world.entity_mut(entity).insert(derived);
        Der {
            rctx_entity: entity,
            p: PhantomData,
        }
    }
}

#[derive(Component)]
pub(crate) struct Observable<T> {
    data: T,
    subscribers: HashSet<Entity>,
}
