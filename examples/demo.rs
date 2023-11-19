use std::time::Instant;

use bevy_app::{prelude::*, ScheduleRunnerPlugin};
use bevy_ecs::prelude::*;
use bevy_rx::prelude::*;

fn main() {
    App::new()
        .add_plugins((ScheduleRunnerPlugin::run_once(), ReactiveExtensionsPlugin))
        .add_systems(Startup, setup)
        .add_systems(Update, update)
        .run()
}

#[derive(Debug, Component, Clone, Copy)]
struct Button {
    active: Signal<bool>,
}

#[derive(Debug, Component, Clone, Copy)]
struct Lock {
    unlocked: Calc<bool>,
}

impl Lock {
    /// A lock will only unlock if both of its buttons are active
    fn two_buttons(buttons: (&bool, &bool)) -> bool {
        *buttons.0 && *buttons.1
    }
}

fn setup(mut commands: Commands, mut reactor: Reactor) {
    let button1 = Button {
        active: reactor.signal(false),
    };
    let button2 = Button {
        active: reactor.signal(false),
    };
    let lock = Lock {
        unlocked: reactor.calc((button1.active, button2.active), Lock::two_buttons),
    };
    commands.spawn(lock);
    commands.spawn(button1);
    commands.spawn(button2);
}

fn update(mut reactor: Reactor, buttons: Query<&Button>, lock: Query<&Lock>) {
    // Signals and derivations from the ECS
    let mut buttons = buttons.iter().copied();
    let [button1, button2] = [buttons.next().unwrap(), buttons.next().unwrap()];
    let lock1 = *lock.single();
    reactor.send_signal(button1.active, true);

    let start = Instant::now();
    reactor.send_signal(button2.active, true);
    println!("Sending 1 signal = {:#?}", start.elapsed());

    assert!(reactor.read(lock1.unlocked));

    let start = Instant::now();
    for _ in 0..1_000_000 {
        reactor.send_signal(button1.active, true); // diffing prevents triggering a recompute
    }
    println!(
        "Sending 1,000,000 signals with same value = {:#?}/iter",
        start.elapsed() / 1_000_000
    );

    let start = Instant::now();
    for i in 1..=1_000_000 {
        reactor.send_signal(button1.active, i % 2 == 0);
    }
    println!(
        "Sending 1,000,000 signals with alternating value = {:#?}/iter",
        start.elapsed() / 1_000_000
    );

    let local_signal = reactor.signal(false); // We can add a new signal locally

    let lock2 = reactor.calc((button1.active, local_signal), Lock::two_buttons); // Local and ECS
    reactor.send_signal(local_signal, true);
    let start = Instant::now();
    for _ in 0..1_000_000 {
        if !reactor.read(lock2) {
            dbg!("nope");
        }
    }
    println!(
        "Reading a derivation 1,000,000 times = {:#?}/iter",
        start.elapsed() / 1_000_000
    );

    assert!(reactor.read(lock2));
}
