use std::marker::PhantomData;

use bevy_ecs::prelude::*;

use crate::{observable::ObservableData, Observable, ReactiveContext};

/// An reactive component that can be subscribed to, and whose value can only be accessed through
/// the [`ReactiveContext`].
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

impl<T: Send + Sync + PartialEq> Signal<T> {
    pub(crate) fn new(rctx: &mut ReactiveContext, initial_value: T) -> Self {
        Self {
            reactor_entity: ObservableData::new(rctx, initial_value),
            p: PhantomData,
        }
    }

    pub fn read<'r>(&self, rctx: &'r mut ReactiveContext) -> &'r T {
        rctx.world
            .get::<ObservableData<T>>(self.reactor_entity)
            .unwrap()
            .data()
    }

    /// See [`ReactiveContext::send_signal`].
    #[inline]
    pub fn send(&self, rctx: &mut ReactiveContext, value: T) {
        ObservableData::send_signal(&mut rctx.world, self.reactor_entity, value)
    }
}
