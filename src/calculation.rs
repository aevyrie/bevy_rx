use std::marker::PhantomData;

use bevy_ecs::prelude::*;
use bevy_utils::all_tuples_with_size;

use crate::{Observable, ObservableData, ReactiveContext};

/// A reactive component whose value is recalculated automatically, and can only be read through the
/// [`ReactiveContext`].
#[derive(Debug, Component)]
pub struct Calc<T: Send + Sync + 'static> {
    pub(crate) reactor_entity: Entity,
    pub(crate) p: PhantomData<T>,
}

impl<T: Send + Sync + PartialEq> Observable for Calc<T> {
    type Data = T;
    fn reactive_entity(&self) -> Entity {
        self.reactor_entity
    }
}

impl<T: Send + Sync> Clone for Calc<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: Send + Sync> Copy for Calc<T> {}

impl<T: PartialEq + Send + Sync> Calc<T> {
    pub fn new<D: CalcQuery<T>>(
        rctx: &mut ReactiveContext,
        input_deps: D,
        derive_fn: (impl Fn(D::Query<'_>) -> T + Send + Sync + Clone + 'static),
    ) -> Self {
        let entity = rctx.world.spawn_empty().id();
        let mut derived = CalcFunction::new(entity, input_deps, derive_fn);
        derived.execute(&mut rctx.world, &mut Vec::new());
        rctx.world.entity_mut(entity).insert(derived);
        Self {
            reactor_entity: entity,
            p: PhantomData,
        }
    }

    pub fn read<'r>(&self, rctx: &'r mut ReactiveContext) -> &'r T {
        rctx.world
            .get::<ObservableData<T>>(self.reactor_entity)
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
pub(crate) struct CalcFunction {
    function: Box<dyn DeriveFn>,
}

trait DeriveFn: Send + Sync + FnMut(&mut World, &mut Vec<Entity>) {}
impl<T: Send + Sync + FnMut(&mut World, &mut Vec<Entity>)> DeriveFn for T {}

impl CalcFunction {
    pub(crate) fn new<C: Send + Sync + PartialEq + 'static, D: CalcQuery<C> + 'static>(
        entity: Entity,
        input_deps: D,
        derive_fn: (impl Fn(D::Query<'_>) -> C + Send + Sync + Clone + 'static),
    ) -> Self {
        let function = move |world: &mut World, stack: &mut Vec<Entity>| {
            let computed_value = D::read_and_derive(world, entity, derive_fn.clone(), input_deps);
            if let Some(computed_value) = computed_value {
                ObservableData::update_value(world, stack, entity, computed_value);
            }
        };
        let function = Box::new(function);
        Self { function }
    }

    pub(crate) fn execute(&mut self, world: &mut World, stack: &mut Vec<Entity>) {
        (self.function)(world, stack);
    }
}

/// Implemented on tuples to be used for querying
pub trait CalcQuery<T>: Copy + Send + Sync + 'static {
    type Query<'a>;
    fn read_and_derive(
        world: &mut World,
        reader: Entity,
        derive_fn: impl Fn(Self::Query<'_>) -> T,
        input_deps: Self,
    ) -> Option<T>;
}

macro_rules! impl_CalcQuery {
    ($N: expr, $(($T: ident, $I: ident)),*) => {
        impl<$($T: Observable), *, D> CalcQuery<D> for ($($T,)*) {
            type Query<'a> = ($(&'a $T::Data,)*);

            fn read_and_derive(
                world: &mut World,
                reader: Entity,
                derive_fn: impl Fn(Self::Query<'_>) -> D,
                entities: Self,
            ) -> Option<D> {
                let ($($I,)*) = entities;
                let entities = [$($I.reactive_entity(),)*];

                // Note this is left to unwrap intentionally. If aliased mutability happens, this is
                // an error and should panic. If we were to early exit here, it would lead to
                // harder-to-debug errors down the line.
                let [$(mut $I,)*] = world.get_many_entities_mut(entities).unwrap();

                $($I.get_mut::<ObservableData<$T::Data>>()?.subscribe(reader);)*

                Some(derive_fn((
                    $($I.get::<ObservableData<$T::Data>>()?.data(),)*
                )))
            }
        }
    }
}

all_tuples_with_size!(impl_CalcQuery, 1, 32, T, s);
