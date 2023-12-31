use std::marker::PhantomData;

use bevy_ecs::prelude::*;

use crate::{observable::RxObservableData, Observable, ReactiveContext};

/// A reactive component that can updated with new values or read through the [`ReactiveContext`].
#[derive(Debug, Component)]
pub struct Signal<T: Send + Sync + 'static> {
    reactor_entity: Entity,
    p: PhantomData<T>,
}

impl<T: Send + Sync + PartialEq> Observable for Signal<T> {
    type DataType = T;
    fn reactive_entity(&self) -> Entity {
        self.reactor_entity
    }
}

impl<T: Send + Sync + PartialEq> Clone for Signal<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: Send + Sync + PartialEq> Copy for Signal<T> {}

impl<T: Clone + Send + Sync + PartialEq> Signal<T> {
    pub(crate) fn new<S>(rctx: &mut ReactiveContext<S>, initial_value: T) -> Self {
        Self {
            reactor_entity: RxObservableData::new(rctx, initial_value),
            p: PhantomData,
        }
    }

    pub fn read<'r, S>(&self, rctx: &'r mut ReactiveContext<S>) -> &'r T {
        rctx.reactive_state
            .get::<RxObservableData<T>>(self.reactor_entity)
            .unwrap()
            .data()
    }

    /// See [`ReactiveContext::send_signal`].
    #[inline]
    pub fn send<S>(&self, rctx: &mut ReactiveContext<S>, value: T) {
        RxObservableData::send_signal(&mut rctx.reactive_state, self.reactor_entity, value)
    }
}
