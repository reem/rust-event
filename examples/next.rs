extern crate event;

fn main() {
    event::next(|| { println!("Hello World!"); });
    event::run();
}

