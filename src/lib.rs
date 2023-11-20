/// Experimental reactivity extension for bevy.
///
/// This plugin holds a reactive graph inside a resource, allowing you to define reactive
/// relationships, and execute them synchronously. This makes it possible to (effectively) mutate
/// reactive components in the bevy world, even when you don't have mutable access to them. We
/// aren't breaking rust's aliasing rules because the components aren't actually being mutated -
/// [`Signal`], [`Derived`] - are actually lightweight handles to data inside the resource.
///
/// The only way to access the data is through the context, or using the slightly less verbose
/// [`Reactor`] system param.
///
/// This makes it possible to define a complex network of signals, derived values, and effects, and
/// execute them reactively to completion without worrying about frame delays seen with event
/// propagation or component mutation.
use std::ops::{Deref, DerefMut};

use bevy_app::PostUpdate;
use bevy_ecs::{prelude::*, system::SystemParam};
use effect::{Effect, RxDeferredEffect, RxDeferredEffects};
use memo::MemoQuery;
use observable::{Observable, RxObservableData};
use prelude::Memo;
use signal::Signal;

pub mod effect;
pub mod memo;
pub mod observable;
pub mod signal;

pub mod prelude {
    pub use crate::{
        memo::Memo, signal::Signal, ReactiveContext, ReactiveExtensionsPlugin, Reactor,
    };
}

pub struct ReactiveExtensionsPlugin;

impl ReactiveExtensionsPlugin {
    fn apply_deferred_effects(world: &mut World) {
        world.resource_scope::<ReactiveContext, _>(|world, mut rctx| {
            let mut effects: Vec<_> = std::mem::take(
                rctx.reactive_state
                    .resource_mut::<RxDeferredEffects>()
                    .stack
                    .as_mut(),
            );
            for effect in effects.drain(..) {
                effect(world, &mut rctx.reactive_state)
            }
        })
    }
}

impl bevy_app::Plugin for ReactiveExtensionsPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.init_resource::<ReactiveContext>()
            .add_systems(PostUpdate, Self::apply_deferred_effects);
    }
}

/// A system param to make accessing the [`ReactiveContext`] less verbose.
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

/// Contains all reactive state. A bevy world is used because it makes it easy to store statically
/// typed data in a type erased container.
#[derive(Resource)]
pub struct ReactiveContext {
    reactive_state: World,
}

impl Default for ReactiveContext {
    fn default() -> Self {
        let mut world = World::default();
        world.init_resource::<RxDeferredEffects>();
        Self {
            reactive_state: world,
        }
    }
}

impl ReactiveContext {
    /// Returns a reference to the current value of the provided observable. The observable is any
    /// reactive handle that has a value, like a [`Signal`] or a [`Derived`].
    pub fn read<T: Send + Sync + PartialEq + 'static, O: Observable<DataType = T>>(
        &mut self,
        observable: O,
    ) -> &T {
        // get the obs data from the world
        // add the reader to the obs data's subs
        self.reactive_state
            .get::<RxObservableData<T>>(observable.reactive_entity())
            .unwrap()
            .data()
    }

    /// Send a signal, and run the reaction graph to completion.
    ///
    /// Potentially expensive operation that will write a value to this [`Signal`]`. This will cause
    /// all reactive subscribers of this observable to recompute their own values, which can cause
    /// all of its subscribers to recompute, etc.
    pub fn send_signal<T: Clone + Send + Sync + PartialEq + 'static>(
        &mut self,
        signal: Signal<T>,
        value: T,
    ) {
        RxObservableData::send_signal(&mut self.reactive_state, signal.reactive_entity(), value)
    }

    pub fn new_signal<T: Clone + Send + Sync + PartialEq + 'static>(
        &mut self,
        initial_value: T,
    ) -> Signal<T> {
        Signal::new(self, initial_value)
    }

    pub fn new_memo<T: Clone + Send + Sync + PartialEq + 'static, C: MemoQuery<T> + 'static>(
        &mut self,
        calculation_query: C,
        derive_fn: (impl Fn(C::Query<'_>) -> T + Send + Sync + Clone + 'static),
    ) -> Memo<T> {
        Memo::new(self, calculation_query, derive_fn)
    }

    pub fn new_deferred_effect<M>(
        &mut self,
        observable: impl Observable,
        effect_system: impl IntoSystem<(), (), M>,
    ) -> Effect {
        Effect::new_deferred(self, observable, effect_system)
    }

    pub fn effect_system(&self, effect: Effect) -> Option<&dyn System<In = (), Out = ()>> {
        self.reactive_state
            .get::<RxDeferredEffect>(effect.reactor_entity)
            .and_then(|effect| effect.system())
    }
}

mod test {
    #[test]
    fn basic() {
        #[derive(Debug, Clone, PartialEq)]
        struct Button {
            active: bool,
        }
        impl Button {
            pub const OFF: Self = Button { active: false };
            pub const ON: Self = Button { active: true };
        }

        #[derive(Debug, Clone, PartialEq)]
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

        let mut reactor = crate::ReactiveContext::default();

        let button1 = reactor.new_signal(Button::OFF);
        let button2 = reactor.new_signal(Button::OFF);
        let lock1 = reactor.new_memo((button1, button2), &Lock::two_buttons);
        assert!(!reactor.read(lock1).unlocked);

        let button3 = reactor.new_signal(Button::OFF);
        let lock2 = reactor.new_memo((button1, button3), &Lock::two_buttons);
        assert!(!reactor.read(lock2).unlocked);

        reactor.send_signal(button1, Button::ON); // Automatically recomputes lock1 & lock2!
        reactor.send_signal(button2, Button::ON);
        for _ in 0..1000 {
            button1.send(&mut reactor, Button::ON); // diffing prevents triggering a recompute
        }
        assert!(lock1.read(&mut reactor).unlocked);

        reactor.send_signal(button3, Button::ON);
        assert!(reactor.read(lock2).unlocked);
    }

    #[test]
    fn nested_derive() {
        let mut reactor = crate::ReactiveContext::default();

        let add = |n: (&f32, &f32)| n.0 + n.1;

        let n1 = reactor.new_signal(1.0);
        let n2 = reactor.new_signal(10.0);
        let n3 = reactor.new_signal(100.0);

        // The following derives use signals as inputs
        let d1 = reactor.new_memo((n1, n2), add); // 1 + 10
        let d2 = reactor.new_memo((n3, n2), add); // 100 + 10

        // This derive uses other derives as inputs
        let d3 = reactor.new_memo((d1, d2), add); // 11 + 110
        assert_eq!(*reactor.read(d3), 121.0);
    }

    #[test]
    fn many_types() {
        #[derive(Debug, Clone, PartialEq)]
        struct Foo(f32);
        #[derive(Debug, Clone, PartialEq)]
        struct Bar(f32);
        #[derive(Debug, Clone, PartialEq)]
        struct Baz(f32);

        let mut reactor = crate::ReactiveContext::default();

        let foo = reactor.new_signal(Foo(1.0));
        let bar = reactor.new_signal(Bar(1.0));

        let baz = reactor.new_memo((foo, bar), |(foo, bar)| Baz(foo.0 + bar.0));

        assert_eq!(reactor.read(baz), &Baz(2.0));
    }

    #[test]
    fn calculate_pi() {
        let mut reactor = crate::ReactiveContext::default();

        let increment = |(n,): (&f64,)| n + 1.0;
        let bailey_borwein_plouffe = |(k, last_value): (&f64, &f64)| {
            last_value
                + 1.0 / (16f64.powf(*k))
                    * (4.0 / (8.0 * k + 1.0)
                        - 2.0 / (8.0 * k + 4.0)
                        - 1.0 / (8.0 * k + 5.0)
                        - 1.0 / (8.0 * k + 6.0))
        };

        let start = bevy_utils::Instant::now();
        // Initial values
        let k_0 = reactor.new_signal(0.0);
        let iter_0 = reactor.new_signal(0.0);
        // Scratch space handles to build graph
        let mut iteration = reactor.new_memo((k_0, iter_0), bailey_borwein_plouffe);
        let mut k = reactor.new_memo((k_0,), increment);
        for _ in 0..100_000_000 {
            iteration = reactor.new_memo((k, iteration), bailey_borwein_plouffe);
            k = reactor.new_memo((k,), increment);
        }
        println!(
            "({:#?}) PI: {:.32}",
            start.elapsed(),
            reactor.read(iteration)
        );

        let start = bevy_utils::Instant::now();
        reactor.send_signal(k_0, f64::EPSILON);
        println!("Recomputing PI took = {:#?}", start.elapsed());
    }
}
