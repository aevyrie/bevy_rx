use bevy_ecs::{prelude::*, system::BoxedSystem};

/// A reactive component that makes changes to the bevy [`World`] by running a system only when the
/// values it queries change.
#[derive(Debug, Component)]
pub struct Effect {
    pub(crate) reactor_entity: Entity,
}

#[derive(Resource)]
pub(crate) struct RxDeferredEffects {
    pub(crate) stack: Vec<Box<dyn FnOnce(&mut World) + Send + Sync>>,
}

#[derive(Debug, Component)]
pub(crate) struct RxEffect<T: 'static> {
    pub(crate) system: EffectSystem<T>,
}

impl<T> RxEffect<T> {
    pub fn run(&mut self, main_world: &mut World, value: T) {
        self.system.run(main_world, value);
    }
}

#[derive(Default, Debug)]
pub enum EffectSystem<T: 'static> {
    #[default]
    Empty,
    New(BoxedSystem<In<T>>),
    Initialized(BoxedSystem<In<T>>),
}

impl<T> EffectSystem<T> {
    pub(crate) fn run(&mut self, world: &mut World, value: T) {
        let mut system = match std::mem::take(self) {
            EffectSystem::Empty => return,
            EffectSystem::New(mut system) => {
                system.initialize(world);
                system
            }
            EffectSystem::Initialized(system) => system,
        };
        system.run(In(value), world);
        system.apply_deferred(world);
        *self = EffectSystem::Initialized(system);
    }
}

// how should reactive callbacks be run?
// needs to be in an exclusive system
// only run when watched value changes
