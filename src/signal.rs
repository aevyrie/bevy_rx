use std::marker::PhantomData;

use bevy_ecs::prelude::*;
use bevy_utils::HashSet;

use crate::{Observable, ReactiveContext};

/// An observable component that can only be accessed through the [`ReactiveContext`].
#[derive(Debug, Component)]
pub struct Signal<T: Send + Sync + 'static> {
    pub(crate) reactor_entity: Entity,
    pub(crate) p: PhantomData<T>,
}

impl<T: Send + Sync + PartialEq + 'static> Observable<T> for Signal<T> {
    fn reactor_entity(&self) -> Entity {
        self.reactor_entity
    }
}

impl<T: Send + Sync + PartialEq + 'static> Clone for Signal<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: Send + Sync + PartialEq + 'static> Copy for Signal<T> {}

impl<T: Send + Sync + PartialEq + 'static> Signal<T> {
    pub fn read<'r>(&self, rctx: &'r mut ReactiveContext) -> &'r T {
        &rctx
            .world
            .get::<SignalData<T>>(self.reactor_entity)
            .unwrap()
            .data
    }

    /// See [`ReactiveContext::send_signal`].
    #[inline]
    pub fn send(&self, rctx: &mut ReactiveContext, value: T) {
        SignalData::send_signal(&mut rctx.world, self.reactor_entity, value)
    }
}

/// The signal component used in the reactive world, which holds the actual data. [`Signal`] is what
/// users of this plugin use, which is just a lightweight handle to access this mirror component.
#[derive(Component)]
pub(crate) struct SignalData<T> {
    data: T,
    subscribers: HashSet<Entity>,
}

impl<T: Send + Sync + 'static> SignalData<T> {
    pub(crate) fn new(data: T) -> Self {
        Self {
            data,
            subscribers: Default::default(),
        }
    }

    pub(crate) fn add_subscriber(&mut self, entity: Entity) {
        self.subscribers.insert(entity);
    }

    pub(crate) fn data(&self) -> &T {
        &self.data
    }
}

impl<T: PartialEq + Send + Sync + 'static> SignalData<T> {
    /// Write data to the supplied entity. The [`Observable`] component will be added if it
    /// is missing.
    #[inline]
    pub(crate) fn send_signal(world: &mut World, entity: Entity, value: T) {
        if let Some(mut signal) = world.get_mut::<SignalData<T>>(entity) {
            if signal.data == value {
                return;
            }
            signal.data = value;
            let mut subscribers = std::mem::take(&mut signal.subscribers);
            for sub in subscribers.drain() {
                let mut derived = world
                    .entity_mut(sub)
                    .take::<crate::derived::DerivedData>()
                    .unwrap();
                derived.execute(world);
                world.entity_mut(sub).insert(derived);
            }
        } else {
            world.entity_mut(entity).insert(SignalData {
                data: value,
                subscribers: Default::default(),
            });
        }
    }
}
