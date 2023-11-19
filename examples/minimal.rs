fn main() {
    let mut reactor = bevy_rx::ReactiveContext::default();

    // Signals are statically typed values that can be observed by calculations or effects.
    let first_name = reactor.new_signal("Jane".to_string());
    let last_name = reactor.new_signal("Doe".to_string());
    let age = reactor.new_signal(45);

    // Calculations can take any observable as input, and apply a calculation - this can be a
    // closure or a function. Here we define a closure as a variable we could reuse:
    let join_with_space = |(s1, s2): (&String, &String)| format!("{s1} {s2}");
    let full_name = reactor.new_memo((first_name, last_name), join_with_space);

    // We can also define the calculation as a function
    let welcome = reactor.new_memo((full_name, age), welcome_message);

    assert_eq!(
        dbg!(reactor.read(welcome)),
        "Welcome Jane Doe, you are 45 years old."
    );

    reactor.send_signal(first_name, "Katie".to_string());

    assert_eq!(
        dbg!(reactor.read(welcome)),
        "Welcome Katie Doe, you are 45 years old."
    );
}

fn welcome_message((name, age): (&String, &i32)) -> String {
    format!("Welcome {name}, you are {age} years old.")
}
