use bevy_rx::{self, *};

struct Button {
    active: bool,
}

#[derive(Debug)]
struct Lock {
    unlocked: bool,
}

fn main() {
    let mut rctx = ReactiveContext::default();

    let button1 = rctx.add_observable(Button { active: false });
    let button2 = rctx.add_observable(Button { active: false });

    let lock = rctx.add_derived::<Lock, (Button, Button)>((button1, button2), |(b1, b2)| Lock {
        unlocked: b1.active && b2.active,
    });

    assert!(!lock.read(&mut rctx).unlocked);

    button1.write_and_react(&mut rctx, Button { active: true });
    button2.write_and_react(&mut rctx, Button { active: true });

    assert!(lock.read(&mut rctx).unlocked);
}

// fn recompute_lock(button: (&Button, &Button)) -> Lock {
//     Lock {
//         unlocked: button.0.active && button.1.active,
//     }
// }
