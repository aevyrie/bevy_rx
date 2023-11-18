use bevy_ecs::prelude::*;

use crate::ReactiveContext;

/// The core reactive primitive that holds data, and a list of subscribers that are invoked when the
/// data changes.
#[derive(Component)]
pub(crate) struct Reactive<T> {
    pub data: T,
    pub subscribers: Vec<Entity>,
}

impl<T: Send + Sync + 'static> Reactive<T> {
    #[allow(clippy::new_ret_no_self)]
    pub(crate) fn new(rctx: &mut ReactiveContext, data: T) -> Entity {
        rctx.world
            .spawn(Self {
                data,
                subscribers: Vec::with_capacity(0),
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

impl<T: PartialEq + Send + Sync + 'static> Reactive<T> {
    /// Update value of this reactive entity, additionally, trigger all subscribers. The
    /// [`Reactive`] component will be added if it is missing.
    pub(crate) fn send_signal(world: &mut World, signal_target: Entity, value: T) {
        if let Some(mut reactive) = world.get_mut::<Reactive<T>>(signal_target) {
            if reactive.data == value {
                return; // Diff the value and early exit if no change.
            }
            reactive.data = value;

            let mut stack = std::mem::take(&mut reactive.subscribers);
            while let Some(sub) = stack.pop() {
                if let Some(mut calculation) = world
                    .entity_mut(sub)
                    .take::<crate::calculated::CalcFunction>()
                {
                    calculation.execute(world, &mut stack);
                    world.entity_mut(sub).insert(calculation);
                }
            }
        } else {
            world.entity_mut(signal_target).insert(Reactive {
                data: value,
                subscribers: Default::default(),
            });
        }
    }
}
