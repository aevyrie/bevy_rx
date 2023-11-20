use bevy_ecs::{prelude::*, system::BoxedSystem};

use crate::{
    observable::{Observable, RxObservableData},
    ReactiveContext,
};

/// A reactive component that makes changes to the bevy [`World`] by running a system only when the
/// values it queries change.
#[derive(Debug, Clone, Copy, Component)]
pub struct Effect {
    pub(crate) reactor_entity: Entity,
}

impl Effect {
    pub fn new_deferred<M>(
        rctx: &mut ReactiveContext,
        observable: impl Observable,
        effect_system: impl IntoSystem<(), (), M>,
    ) -> Self {
        let reactor_entity = observable.reactive_entity();
        rctx.reactive_state
            .entity_mut(reactor_entity)
            .insert(RxDeferredEffect::new(effect_system));

        Self { reactor_entity }
    }

    pub fn get<'r>(
        &self,
        rctx: &'r mut ReactiveContext,
    ) -> Option<&'r dyn System<In = (), Out = ()>> {
        rctx.reactive_state
            .get::<RxDeferredEffect>(self.reactor_entity)
            .unwrap()
            .system()
    }
}

/// A function used to run effects via dependency injection.
pub type EffectFn = dyn FnOnce(&mut World, &mut World) + Send + Sync;

/// A stack of side effects that is gathered while systems run and update reactive data. This allows
/// effects to be gathered during normal (non-exclusive) system execution in the user's main world.
/// Once the user wants to execute the side effects, the plugin will need an exclusive system to run
/// the effects in a big batch. This is the "deferred" part of the name.
#[derive(Resource, Default)]
pub(crate) struct RxDeferredEffects {
    pub(crate) stack: Vec<Box<EffectFn>>,
}

impl RxDeferredEffects {
    pub fn push<T: Clone + PartialEq + Send + Sync + 'static>(&mut self, observable: Entity) {
        let effect = Box::new(move |main_world: &mut World, rx_world: &mut World| {
            let Some(value) = rx_world
                .entity_mut(observable)
                .take::<RxObservableData<T>>()
            else {
                return;
            };

            let Some(mut effect) = rx_world.entity_mut(observable).take::<RxDeferredEffect>()
            else {
                rx_world.entity_mut(observable).insert(value);
                return;
            };

            let RxObservableData { data, subscribers } = value;
            main_world.insert_resource(EffectData { value: data });

            effect.run(main_world);

            // Return the observable data back into its original component:
            let data = main_world
                .remove_resource::<EffectData<T>>()
                .expect("EffectData does not exist after running effect. Did you remove it?")
                .value;
            rx_world
                .entity_mut(observable)
                .insert(RxObservableData { data, subscribers });

            // Return the effect system back to its original component:
            rx_world.entity_mut(observable).insert(effect);
        });
        self.stack.push(effect);
    }
}

/// A resource that exists solely to allow [`Effect`]s to gain access to the data they are reacting
/// to, without requiring that data implement `Clone`.
#[derive(Resource)]
pub struct EffectData<T> {
    value: T,
}

impl<T> EffectData<T> {
    pub fn value(&self) -> &T {
        &self.value
    }
}

impl<T> std::ops::Deref for EffectData<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.value()
    }
}

/// A side effect applied to the main world at a deferred sync point, as a reaction to some value
/// changing.
///
/// An effect is an optional component stored alongside observable data
/// ([`crate::RxObservableData`]), and is triggered any time that observable data changes.
#[derive(Debug, Component)]
pub(crate) struct RxDeferredEffect {
    pub(crate) system: EffectSystem,
}

impl RxDeferredEffect {
    pub(crate) fn new<M>(system: impl IntoSystem<(), (), M>) -> Self {
        Self {
            system: EffectSystem::new(system),
        }
    }

    pub(crate) fn run(&mut self, main_world: &mut World) {
        self.system.run(main_world);
    }

    pub fn system(&self) -> Option<&dyn System<In = (), Out = ()>> {
        match &self.system {
            EffectSystem::Empty => None,
            EffectSystem::New(s) | EffectSystem::Initialized(s) => Some(s.as_ref()),
        }
    }
}

#[derive(Default, Debug)]
pub(crate) enum EffectSystem {
    #[default]
    Empty,
    New(BoxedSystem),
    Initialized(BoxedSystem),
}

impl EffectSystem {
    pub(crate) fn new<M>(system: impl IntoSystem<(), (), M>) -> Self {
        Self::New(Box::new(IntoSystem::into_system(system)))
    }

    pub(crate) fn run(&mut self, world: &mut World) {
        let mut system = match std::mem::take(self) {
            EffectSystem::Empty => return,
            EffectSystem::New(mut system) => {
                system.initialize(world);
                system
            }
            EffectSystem::Initialized(system) => system,
        };
        system.run((), world);
        system.apply_deferred(world);
        *self = EffectSystem::Initialized(system);
    }
}
