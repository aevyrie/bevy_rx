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

use bevy_ecs::{prelude::*, system::SystemParam};
use derived::{Derivable, Derived};
use reactive::Reactive;
use signal::Signal;

pub mod derived;
pub mod reactive;
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

/// Generalizes over multiple bevy reactive components the user has access to, that are ultimately
/// just handles containing the entity in the [`ReactiveContext`].
pub trait Observable: Copy + Send + Sync + 'static {
    type Data: PartialEq + Send + Sync + 'static;
    fn reactive_entity(&self) -> Entity;
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
#[derive(Default, Resource)]
pub struct ReactiveContext {
    world: World,
}

impl ReactiveContext {
    /// Returns a reference to the current value of the provided observable. The observable is any
    /// reactive handle that has a value, like a [`Signal`] or a [`Derived`].
    pub fn read<T: Send + Sync + PartialEq + 'static, O: Observable<Data = T>>(
        &mut self,
        observable: O,
    ) -> &T {
        // get the obs data from the world
        // add the reader to the obs data's subs
        self.world
            .get::<Reactive<T>>(observable.reactive_entity())
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
        signal: Signal<T>,
        value: T,
    ) {
        Reactive::send_signal(&mut self.world, signal.reactive_entity(), value)
    }

    pub fn new_signal<T: Send + Sync + PartialEq + 'static>(
        &mut self,
        initial_value: T,
    ) -> Signal<T> {
        Signal::new(self, initial_value)
    }

    pub fn new_derived<C: Send + Sync + PartialEq + 'static, D: Derivable<C> + 'static>(
        &mut self,
        input_deps: D,
        derive_fn: (impl Fn(D::Query<'_>) -> C + Send + Sync + Clone + 'static),
    ) -> Derived<C> {
        Derived::new(self, input_deps, derive_fn)
    }
}

mod test {
    #[test]
    fn basic() {
        #[derive(Debug, PartialEq)]
        struct Button {
            active: bool,
        }
        impl Button {
            pub const OFF: Self = Button { active: false };
            pub const ON: Self = Button { active: true };
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

        let mut reactor = crate::ReactiveContext::default();

        let button1 = reactor.new_signal(Button::OFF);
        let button2 = reactor.new_signal(Button::OFF);
        let lock1 = reactor.new_derived((button1, button2), &Lock::two_buttons);
        assert!(!reactor.read(lock1).unlocked);

        let button3 = reactor.new_signal(Button::OFF);
        let lock2 = reactor.new_derived((button1, button3), &Lock::two_buttons);
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
        let d1 = reactor.new_derived((n1, n2), add); // 1 + 10
        let d2 = reactor.new_derived((n3, n2), add); // 100 + 10

        // This derive uses other derives as inputs
        let d3 = reactor.new_derived((d1, d2), add); // 11 + 110
        assert_eq!(*reactor.read(d3), 121.0);
    }

    #[test]
    fn many_types() {
        #[derive(Debug, PartialEq)]
        struct Foo(f32);
        #[derive(Debug, PartialEq)]
        struct Bar(f32);
        #[derive(Debug, PartialEq)]
        struct Baz(f32);

        let mut reactor = crate::ReactiveContext::default();

        let foo = reactor.new_signal(Foo(1.0));
        let bar = reactor.new_signal(Bar(1.0));

        let baz = reactor.new_derived((foo, bar), |(foo, bar)| Baz(foo.0 + bar.0));

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

        let k_0 = reactor.new_signal(0.0);
        let iter_0 = reactor.new_signal(0.0);
        let mut iteration = reactor.new_derived((k_0, iter_0), bailey_borwein_plouffe);
        let mut k = reactor.new_derived((k_0,), increment);
        for _ in 0..1000 {
            iteration = reactor.new_derived((k, iteration), bailey_borwein_plouffe);
            k = reactor.new_derived((k,), increment);
        }
        println!("PI: {:.32}", reactor.read(iteration));

        let start = bevy_utils::Instant::now();
        reactor.send_signal(k_0, f64::EPSILON);
        println!("Recomputing PI took = {:#?}", start.elapsed());
    }
}
