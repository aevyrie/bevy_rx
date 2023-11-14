// use bevy_app::Plugin;
// use bevy_ecs::prelude::*;

// #[derive(Debug, Clone, Resource, Default)]
// pub struct RxSettings;

// #[derive(Debug)]
// pub struct RxPlugin;

// impl Plugin for RxPlugin {
//     fn build(&self, app: &mut bevy_app::App) {
//         app.init_resource::<RxSettings>();
//     }
// }

// #[derive(Debug, Component)]
// struct Rx<T> {
//     data: T,
//     is_changed: bool,
//     subscribers: Vec<Entity>,
// }

// impl<T> Rx<T> {
//     pub fn set(&mut self, value: T) {
//         self.data = value;
//         self.is_changed = true;
//     }
// }

// // if some component changes, run some function that modifies my component state
// // we dont just want a single component, we want this to be an iterator over the observed component, so we can filter and map.
// // so we have a stream of component updates in, that maps to some stream of output component edits
// // ex I have a door that looks at two

// fn setup(commands: Commands) {
//     commands.spawn((
//         Signal<Switch1>::default(),
//     ))

//     commands.spawn((
//         Signal<Switch2>::default(),
//     ))

//     commands.spawn((
//         Signal<Door>::default(),
//     ))
// }

// pub struct RxCtx {
//     effects: Vec<fn(ctx: &Ctx|) >,
// }

// fn reactivity(mut rtx: ResMut<ReactiveContext>) {
//     let button1: Signal<Button>; // A handle stored in a component
//     let button2: Signal<Button>; // A handle stored in a component

//     // This closure gives read-only access to the rtx, and after running the closure once, returns a
//     // handle to the derived reactive value.
//     let door: Derived<Door> = rtx.add_derived(|rtx|
//         let button1: &Button = rtx.get(button1); // Creates a dependency of this door on this button
//         let button2: &Button = rtx.get(button2); // Creates a dependency of this door on this button
//         Door {
//             unlocked: button1.is_pressed && button2.is_pressed,
//         }
//     );

//     // We can send a signal by dereferencing the handle via the RTX. This should update the value, then run all subscribers.

//     // because Signal::write accepts a closure, we can use rust's closure syntax a few different
//     // ways:
//     button1.write(rtx, |button| button.is_pressed = true ); // define the closure manually
//     button1.write(rtx, |button| button.press()); // just refer to a method that takes `&mut self`
//     button1.write(rtx, Button::press); // shorthand for the above
// }

// // let door: Derived<Door> = rtx.add_derived(
// //     // first closure, specify all *potential* deps, this is the only chance to access rtx
// //     |rtx| (rtx.get(button1), rtx.get(button2), rtx.get(button3)),
// //     // the output of the first closure is used as the input to the second, effect closure
// //     |(button1, button2, button3)| {
// //         let unlocked = if rand() {
// //             button1.is_pressed && button2.is_pressed
// //         } else {
// //             button3.is_pressed
// //         };
// //         Door { unlocked }
// //     ));

// // a less verbose version of the above

// fn reactivity() {
//     let button1 = rtx.add_signal(Button::default());
//     let button2 = rtx.add_signal(Button::default());

//     let door = rtx.add_derived(|rtx|
//         Door::new(button1.read(rtx).is_pressed && button2.read(rtx).is_pressed)
//     );

//     button1.action(rtx, Button::press);
//     button2.action(rtx, Button::press);

//     assert!(door.read(rtx).unlocked)
// }

// fn reactivity(mut rtx: ResMut<ReactiveContext>) {
//     let button1 = rtx.add_signal(Button::default());
//     let button2 = rtx.add_signal(Button::default());

//     let door = rtx.add_derived(|rtx|
//         Door::new(button1.is_pressed(rtx) && button2.is_pressed(rtx))
//     );

//     button1.press(rtx);
//     button2.press(rtx);

//     assert!(door.is_unlocked(rtx))
// }

// // hold list of running reaction fns
// // const context = [];

// // add the reaction to this sub list, and ad this sub list to this fns deps?
// // function subscribe(running, subscriptions) {
// //   subscriptions.add(running);
// //   running.dependencies.add(subscriptions);
// // }

// // export function createSignal(value) {
// //   const subscriptions = new Set();

// // when a signal is read
// // get the latest reaction on the context
// // add the reaction to this sub list, and add this sub list to the reactions deps
// // this way, when a signal is read, we know that the currently running reaction should be added to that signal to run when it is written to. The running reaction will also know all reactions that will be run when the signal updates. These are the ones that ran before it?

// //   const read = () => {
// //     const running = context[context.length - 1];
// //     if (running) subscribe(running, subscriptions);
// //     return value;
// //   };

// // when a signal is written
// // update the value
// // execute all subs

// //   const write = (nextValue) => {
// //     value = nextValue;

// //     for (const sub of [...subscriptions]) {
// //       sub.execute();
// //     }
// //   };
// //   return [read, write];
// // }
