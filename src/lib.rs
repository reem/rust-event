#![feature(macro_rules)]
// #![deny(missing_docs, warnings)]

//! An Event-Loop for Rust.

extern crate mio;

use std::cell::RefCell;
use ioloop::{IoLoop, IoLoopSender, Registration};

pub use ioloop::Handler;
pub use error::{EventResult, EventError};

mod ioloop;
mod error;

thread_local!(static EVENT_LOOP: RefCell<IoLoop> =
              RefCell::new(IoLoop::new()));
thread_local!(static EVENT_LOOP_SENDER: IoLoopSender =
              EVENT_LOOP.with(move |event| event.borrow().channel()));

pub fn register<H: Handler>(handler: H) {
    EVENT_LOOP_SENDER.with(move |events| events.send(Registration::new(box handler)));
}

pub fn start() {
    EVENT_LOOP_SENDER.with(|_| {});
}

// Starts the event loop on this thread.
pub fn run() {
    EVENT_LOOP.with(move |event| event.borrow_mut().run().unwrap())
}

