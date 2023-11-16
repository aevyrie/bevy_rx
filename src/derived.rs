use std::marker::PhantomData;

use bevy_ecs::prelude::*;
use bevy_utils::all_tuples_with_size;

use crate::{Observable, ReactiveContext, Signal, SignalData};

/// A derived component whose value is computed automatically, and can only be read through the
/// [`ReactiveContext`].
#[derive(Debug, Component)]
pub struct Derived<T: Send + Sync + 'static> {
    pub(crate) reactor_entity: Entity,
    pub(crate) p: PhantomData<T>,
}

impl<T: Send + Sync + PartialEq + 'static> Observable<T> for Derived<T> {
    fn reactor_entity(&self) -> Entity {
        self.reactor_entity
    }
}

impl<T: Send + Sync + 'static> Clone for Derived<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: Send + Sync + 'static> Copy for Derived<T> {}

impl<T: Send + Sync + 'static> Derived<T> {
    pub fn read<'r>(&self, rctx: &'r mut ReactiveContext) -> &'r T {
        rctx.world
            .get::<SignalData<T>>(self.reactor_entity)
            .unwrap()
            .data()
    }
}

/// Lives alongside the observable component. This holds a system that can be called without the
/// caller knowing any type information, and will update the associated observable component.
///
/// This component lives in the reactive world and holds the user calculation function. [`Derived`]
/// is what users of this plugin use, which is a lightweight handle to access this mirror component.
#[derive(Component)]
pub(crate) struct DerivedData {
    function: Box<dyn DeriveFn>,
}

pub(crate) trait ObservableData: Send + Sync + PartialEq + 'static {}
impl<T: Send + Sync + PartialEq + 'static> ObservableData for T {}

trait DeriveFn: Send + Sync + 'static + FnMut(&mut World) {}
impl<T: Send + Sync + 'static + FnMut(&mut World)> DeriveFn for T {}

impl DerivedData {
    pub(crate) fn new<C: Send + Sync + PartialEq + 'static, D: Derivable<C> + 'static>(
        derived: Entity,
        input_deps: D,
        derive_fn: fn(D::Query<'_>) -> C,
    ) -> Self {
        let function = move |world: &mut World| {
            let derived_result = D::read_and_derive(world, derived, derive_fn, input_deps);
            // using write here will trigger all subscribers, which in turn can trigger this
            // function through derived functions, traversing the entire graph.
            SignalData::send_signal(world, derived, derived_result);
        };
        let function = Box::new(function);
        Self { function }
    }

    pub(crate) fn execute(&mut self, world: &mut World) {
        (self.function)(world);
    }
}

/// A type that represents a collection of observables, which can be used to compute a derived
/// value.
pub trait Derivable<T>: Copy + Send + Sync + 'static {
    type Query<'a>;
    fn read_and_derive(
        world: &mut World,
        reader: Entity,
        derive_fn: fn(Self::Query<'_>) -> T,
        input_deps: Self,
    ) -> T;
}

macro_rules! impl_derivable {
    ($N: expr, $(($T: ident, $I: ident)),*) => {
        impl<$($T: ObservableData),*, D> Derivable<D> for ($(Signal<$T>,)*) {
            type Query<'a> = ($(&'a $T,)*);

            fn read_and_derive(
                world: &mut World,
                reader: Entity,
                derive_fn: fn(Self::Query<'_>) -> D,
                entities: Self,
            ) -> D {
                let ($($I,)*) = entities;
                let entities = [$($I.reactor_entity,)*];
                let [$(mut $I,)*] = world.get_many_entities_mut(entities).unwrap();

                $($I.get_mut::<SignalData<$T>>().unwrap().add_subscriber(reader);)*

                derive_fn((
                    $($I.get::<SignalData<$T>>().unwrap().data(),)*
                ))
            }
        }
    }
}

all_tuples_with_size!(impl_derivable, 1, 32, T, s);
