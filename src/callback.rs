use bevy_ecs::{prelude::*, system::BoxedSystem};

/// A reactive component that makes changes to the bevy [`World`] by applying a command that runs
/// only when the values it queries change.
#[derive(Debug, Component)]
pub struct ReactiveCallback {
    pub(crate) callback_system: CallbackSystem,
}

impl ReactiveCallback {
    pub fn run(&mut self, world: &mut World) {
        self.callback_system.run(world);
    }
}

#[derive(Default, Debug)]
pub enum CallbackSystem {
    #[default]
    Empty,
    New(BoxedSystem),
    Initialized(BoxedSystem),
}

impl CallbackSystem {
    pub(crate) fn run(&mut self, world: &mut World) {
        let mut system = match std::mem::take(self) {
            CallbackSystem::Empty => return,
            CallbackSystem::New(mut system) => {
                system.initialize(world);
                system
            }
            CallbackSystem::Initialized(system) => system,
        };
        system.run((), world);
        system.apply_deferred(world);
        *self = CallbackSystem::Initialized(system);
    }
}

// how should reactive callbacks be run?
// needs to be in an exclusive system
// only run when watched value changes
