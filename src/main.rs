use bevy_rx::{self, *};

struct Button {
    active: bool,
}

#[derive(Debug)]
struct Lock {
    unlocked: bool,
}

impl Lock {
    fn new(unlocked: bool) -> Self {
        Self { unlocked }
    }

    fn compute_from_buttons(buttons: (&Button, &Button)) -> Self {
        Self::new(buttons.0.active && buttons.1.active)
    }
}

fn main() {
    let mut rctx = ReactiveContext::default();

    let button1 = rctx.add_observable(Button { active: false });
    let button2 = rctx.add_observable(Button { active: false });

    let lock = rctx.add_derived((button1, button2), Lock::compute_from_buttons);

    assert!(!lock.read(&mut rctx).unlocked);

    button1.write_and_react(&mut rctx, Button { active: true });
    button2.write_and_react(&mut rctx, Button { active: true });

    assert!(lock.read(&mut rctx).unlocked);
}
