//! Requirements
//!
//! 1. Reactive components must be typed
//! 2. Reactive update must be synchronous and run to completion
//!
//! Can side effects be synchronous if the update is run in an exclusive system?

use std::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

use bevy_ecs::{prelude::*, system::SystemParam};
use derived::{Derivable, Derived, DerivedData};
use signal::{Signal, SignalData};

pub mod derived;
pub mod signal;

pub mod prelude {
    pub use crate::{
        derived::Derived, signal::Signal, ReactiveContext, ReactiveExtensionsPlugin, Reactor,
    };
}

pub struct ReactiveExtensionsPlugin;
impl bevy_app::Plugin for ReactiveExtensionsPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.init_resource::<ReactiveContext>();
    }
}

pub trait Observable<T> {
    fn reactor_entity(&self) -> Entity;
}

#[derive(SystemParam)]
pub struct Reactor<'w>(ResMut<'w, ReactiveContext>);
impl<'w> Deref for Reactor<'w> {
    type Target = ReactiveContext;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<'w> DerefMut for Reactor<'w> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Default, Resource)]
pub struct ReactiveContext {
    world: World,
}

impl ReactiveContext {
    /// Returns a reference to the current value of the provided observable.
    pub fn read<T: Send + Sync + PartialEq + 'static>(
        &mut self,
        observable: impl Observable<T>,
    ) -> &T {
        // get the obs data from the world
        // add the reader to the obs data's subs
        self.world
            .get::<SignalData<T>>(observable.reactor_entity())
            .unwrap()
            .data()
    }

    /// Send a signal, and run the reaction graph to completion.
    ///
    /// Potentially expensive operation that will write a value to this [`Signal`]`. This will cause
    /// all reactive subscribers of this observable to recompute their own values, which can cause
    /// all of its subscribers to recompute, etc.
    pub fn send_signal<T: Send + Sync + PartialEq + 'static>(
        &mut self,
        observable: Signal<T>,
        value: T,
    ) {
        observable.send_signal(self, value);
    }

    pub fn add_signal<T: Send + Sync + PartialEq + 'static>(
        &mut self,
        initial_value: T,
    ) -> Signal<T> {
        let rctx_entity = self.world.spawn(SignalData::new(initial_value)).id();
        Signal {
            reactor_entity: rctx_entity,
            p: PhantomData,
        }
    }

    pub fn add_derived<C: Send + Sync + PartialEq + 'static, D: Derivable<C> + 'static>(
        &mut self,
        input_deps: D,
        derive_fn: fn(D::Query<'_>) -> C,
    ) -> Derived<C> {
        let entity = self.world.spawn_empty().id();
        let mut derived = DerivedData::new(entity, input_deps, derive_fn);
        derived.execute(&mut self.world);
        self.world.entity_mut(entity).insert(derived);
        Derived {
            reactor_entity: entity,
            p: PhantomData,
        }
    }
}

mod test {

    #[test]
    fn basic() {
        use crate::ReactiveContext;

        #[derive(Debug, PartialEq)]
        struct Button {
            active: bool,
        }
        impl Button {
            const OFF: Self = Button { active: false };
            const ON: Self = Button { active: true };
        }

        #[derive(Debug, PartialEq)]
        struct Lock {
            unlocked: bool,
        }

        impl Lock {
            /// A lock will only unlock if both of its buttons are active
            fn two_buttons(buttons: (&Button, &Button)) -> Self {
                let unlocked = buttons.0.active && buttons.1.active;
                println!("Recomputing lock. Unlocked: {unlocked}");
                Self { unlocked }
            }
        }

        let mut reactor = ReactiveContext::default();

        let button1 = reactor.add_signal(Button::OFF);
        let button2 = reactor.add_signal(Button::OFF);
        let lock1 = reactor.add_derived((button1, button2), Lock::two_buttons);
        assert!(!reactor.read(lock1).unlocked);

        let button3 = reactor.add_signal(Button::OFF);
        let lock2 = reactor.add_derived((button1, button3), Lock::two_buttons);
        assert!(!reactor.read(lock2).unlocked);

        reactor.send_signal(button1, Button::ON); // Automatically recomputes lock1 & lock2!
        reactor.send_signal(button2, Button::ON);
        for _ in 0..1000 {
            button1.send_signal(&mut reactor, Button::ON); // diffing prevents triggering a recompute
        }
        assert!(lock1.read(&mut reactor).unlocked);

        reactor.send_signal(button3, Button::ON);
        assert!(reactor.read(lock2).unlocked);
    }
}
