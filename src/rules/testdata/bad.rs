fn process(input: i32) -> i32 {
    let scaled = dbg!(input * 2);
    scaled + 1
}

// TODO wire this into the loader
fn done() -> bool {
    println!("not flagged: deliberate CLI output");
    true
}
