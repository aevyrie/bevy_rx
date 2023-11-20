use bevy_ecs::prelude::*;

use crate::{
    effect::{RxDeferredEffect, RxDeferredEffects},
    ReactiveContext,
};

/// Generalizes over multiple bevy reactive components the user has access to, that are ultimately
/// just handles containing the entity in the [`ReactiveContext`].
pub trait Observable: Copy + Send + Sync + 'static {
    type DataType: PartialEq + Send + Sync + 'static;
    fn reactive_entity(&self) -> Entity;
}

/// The core reactive primitive that holds data, and a list of subscribers that are invoked when the
/// data changes.
#[derive(Component)]
pub(crate) struct RxObservableData<T> {
    pub data: T,
    pub subscribers: Vec<Entity>,
}

impl<T: Send + Sync + 'static> RxObservableData<T> {
    #[allow(clippy::new_ret_no_self)]
    pub(crate) fn new(rctx: &mut ReactiveContext, data: T) -> Entity {
        rctx.reactive_state
            .spawn(Self {
                data,
                subscribers: Vec::new(),
            })
            .id()
    }

    pub(crate) fn subscribe(&mut self, entity: Entity) {
        self.subscribers.push(entity);
    }

    pub(crate) fn data(&self) -> &T {
        &self.data
    }
}

impl<T: Clone + PartialEq + Send + Sync + 'static> RxObservableData<T> {
    /// Update the reactive value, and push subscribers onto the stack.
    pub fn update_value(
        rx_world: &mut World,
        stack: &mut Vec<Entity>,
        observable: Entity,
        value: T,
    ) {
        if let Some(mut reactive) = rx_world.get_mut::<RxObservableData<T>>(observable) {
            if reactive.data == value {
                return; // Diff the value and early exit if no change.
            }
            reactive.data = value.clone();
            // Remove all subscribers from this entity. If any of these subscribers end up
            // using this data, they will resubscribe themselves. This is the
            // auto-unsubscribe part of the reactive implementation.
            //
            // We push these subscribers on the stack, so that they can be executed, just
            // like this one was. We use a stack instead of recursion to avoid stack
            // overflow.
            stack.append(&mut reactive.subscribers);
        } else {
            rx_world.entity_mut(observable).insert(RxObservableData {
                data: value.clone(),
                subscribers: Default::default(),
            });
        }
        if rx_world.get_mut::<RxDeferredEffect>(observable).is_some() {
            rx_world
                .resource_mut::<RxDeferredEffects>()
                .push::<T>(observable);
        }
    }
    /// Update value of this reactive entity, additionally, trigger all subscribers. The
    /// [`Reactive`] component will be added if it is missing.
    pub(crate) fn send_signal(world: &mut World, signal_target: Entity, value: T) {
        let mut stack = Vec::new();

        Self::update_value(world, &mut stack, signal_target, value);

        while let Some(sub) = stack.pop() {
            if let Some(mut calculation) = world.entity_mut(sub).take::<crate::memo::RxMemo>() {
                calculation.execute(world, &mut stack);
                world.entity_mut(sub).insert(calculation);
            }
        }
    }
}
