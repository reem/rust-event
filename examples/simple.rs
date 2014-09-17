#![feature(unboxed_closures)]

extern crate event;
extern crate typemap;

use event::{EventQueue, Event};
use typemap::Assoc;

struct ConnectionCreator;

#[deriving(Show)]
struct Connection;

impl Assoc<Connection> for Connection {}

impl ConnectionCreator {
    /// Triggers a Connection
    fn connect(&self) {
        Event::new(Connection).trigger::<Connection>();
    }
}

fn main() {
    // Initialize the event queue.
    EventQueue::new();

    // Set up our handler.
    event::on::<Connection, Connection>(box() (|&: box conn: Box<Connection>| {
        println!("Made connection: {}", conn);
    }) as event::Handler<Connection>);

    // Create our connecter.
    let connecter = ConnectionCreator;

    // Create 100 connections - creating a connection should
    // block as little as possible, and could use an async io
    // library like mio to make non-blocking feasible.
    for i in range(0u, 1000) {
        connecter.connect();
    }

    // Spawn 5 threads to handle connections
    for i in range(0u, 5) {
        event::spawn(event::dedicate);
    }
}

