extern crate event;

use std::time::duration::Duration;

fn main() {
    event::interval(|| { println!("Hello World!"); }, Duration::milliseconds(500));
    event::run()
}

