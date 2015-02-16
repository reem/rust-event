extern crate event;

fn main() {
    event::next(|| { println!("Hello World!"); }).unwrap();
    event::run().unwrap();
}

