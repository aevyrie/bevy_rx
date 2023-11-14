use bevy_rx::*;
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
    fn from_two_buttons(buttons: (&Button, &Button)) -> Self {
        let unlocked = buttons.0.active && buttons.1.active;
        println!("Recomputing lock. Unlocked: {unlocked}");
        Self { unlocked }
    }
}

fn main() {
    let mut rctx = ReactiveContext::default();

    let button1 = rctx.add_observable(Button::OFF);
    let button2 = rctx.add_observable(Button::OFF);
    let lock1 = rctx.add_derived((button1, button2), Lock::from_two_buttons);
    assert!(!lock1.read(&mut rctx).unlocked);

    let button3 = rctx.add_observable(Button::OFF);
    let lock2 = rctx.add_derived((button1, button3), Lock::from_two_buttons);
    assert!(!lock2.read(&mut rctx).unlocked);

    button1.write_and_react(&mut rctx, Button::ON); // Automatically recomputes lock1 & lock2!
    button2.write_and_react(&mut rctx, Button::ON);
    for _ in 0..1000 {
        button1.write_and_react(&mut rctx, Button::ON); // diffing prevents triggering a recompute
    }
    assert!(lock1.read(&mut rctx).unlocked);

    button3.write_and_react(&mut rctx, Button::ON);
    assert!(lock2.read(&mut rctx).unlocked);
}
