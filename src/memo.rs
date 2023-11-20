use std::marker::PhantomData;

use bevy_ecs::prelude::*;
use bevy_utils::all_tuples_with_size;

use crate::{Observable, ReactiveContext, RxObservableData};

/// A reactive value that is automatically recalculated and memoized (cached).
///
/// The value can only be read through the [`ReactiveContext`].
#[derive(Debug, Component)]
pub struct Memo<T: Send + Sync + 'static> {
    pub(crate) reactor_entity: Entity,
    pub(crate) p: PhantomData<T>,
}

impl<T: Send + Sync + PartialEq> Observable for Memo<T> {
    type DataType = T;
    fn reactive_entity(&self) -> Entity {
        self.reactor_entity
    }
}

impl<T: Send + Sync> Clone for Memo<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: Send + Sync> Copy for Memo<T> {}

impl<T: Clone + PartialEq + Send + Sync> Memo<T> {
    pub fn new<D: MemoQuery<T>>(
        rctx: &mut ReactiveContext,
        input_deps: D,
        derive_fn: (impl Fn(D::Query<'_>) -> T + Send + Sync + Clone + 'static),
    ) -> Self {
        let entity = rctx.reactive_state.spawn_empty().id();
        let mut derived = RxMemo::new(entity, input_deps, derive_fn);
        derived.execute(&mut rctx.reactive_state, &mut Vec::new());
        rctx.reactive_state.entity_mut(entity).insert(derived);
        Self {
            reactor_entity: entity,
            p: PhantomData,
        }
    }

    pub fn read<'r>(&self, rctx: &'r mut ReactiveContext) -> &'r T {
        rctx.reactive_state
            .get::<RxObservableData<T>>(self.reactor_entity)
            .unwrap()
            .data()
    }
}

/// A reactive calculation that is run on observable data, and memoized (cached).
///
/// This component lives in the reactive world and holds the user calculation function. [`Memo`] is
/// the user-facing counterpart in the main world, which is a lightweight handle to access this
/// mirror component.
///
/// This component is expected to be on an entity with an [`crate::RxObservableData`] component. The
/// contained function can be called without the caller knowing any type information, and will
/// update the associated [`RxObservableData`] component.
#[derive(Component)]
pub(crate) struct RxMemo {
    function: Box<dyn DeriveFn>,
}

trait DeriveFn: Send + Sync + FnMut(&mut World, &mut Vec<Entity>) {}
impl<T: Send + Sync + FnMut(&mut World, &mut Vec<Entity>)> DeriveFn for T {}

impl RxMemo {
    pub(crate) fn new<C: Clone + Send + Sync + PartialEq + 'static, D: MemoQuery<C> + 'static>(
        entity: Entity,
        input_deps: D,
        derive_fn: (impl Fn(D::Query<'_>) -> C + Clone + Send + Sync + 'static),
    ) -> Self {
        let function = move |world: &mut World, stack: &mut Vec<Entity>| {
            let computed_value = D::read_and_derive(world, entity, derive_fn.clone(), input_deps);
            if let Some(computed_value) = computed_value {
                RxObservableData::update_value(world, stack, entity, computed_value);
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
pub trait MemoQuery<T>: Copy + Send + Sync + 'static {
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
        impl<$($T: Observable), *, D> MemoQuery<D> for ($($T,)*) {
            type Query<'a> = ($(&'a $T::DataType,)*);

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

                $($I.get_mut::<RxObservableData<$T::DataType>>()?.subscribe(reader);)*

                Some(derive_fn((
                    $($I.get::<RxObservableData<$T::DataType>>()?.data(),)*
                )))
            }
        }
    }
}

all_tuples_with_size!(impl_CalcQuery, 1, 32, T, s);
