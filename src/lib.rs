#![feature(macro_rules, unboxed_closures)]
// #![deny(missing_docs, warnings)]

//! An Event-Loop for Rust.

extern crate mio;

use std::cell::RefCell;
use std::time::duration::Duration;
use ioloop::{IoLoop, IoLoopSender, Registration};

pub use handler::{Handler, ClosureHandler};
pub use error::{EventResult, EventError};

mod ioloop;
mod error;
mod handler;

thread_local!(static EVENT_LOOP: RefCell<IoLoop> =
              RefCell::new(IoLoop::new()));
thread_local!(static EVENT_LOOP_SENDER: IoLoopSender =
              EVENT_LOOP.with(move |event| event.borrow().channel()));

pub fn register<H: Handler>(handler: H) {
    EVENT_LOOP_SENDER.with(move |events| events.send(Registration::new(box handler)));
}

pub fn timeout<F: FnOnce() + 'static>(callback: F, timeout: Duration) {
    EVENT_LOOP_SENDER.with(move |events| events.send(Registration::timeout(callback, timeout)));
}

pub fn next<F: FnOnce() + 'static>(callback: F) {
    EVENT_LOOP_SENDER.with(move |events| events.send(Registration::next(callback)));
}

pub fn start() {
    EVENT_LOOP_SENDER.with(|_| {});
}

pub fn interval<F: FnMut() + Send>(callback: F, delay: Duration) {
    struct Interval<F> {
        callback: F,
        delay: Duration
    }

    impl<F: FnMut() + Send()> FnOnce() for Interval<F> {
        extern "rust-call" fn call_once(mut self, _: ()) {
            (self.callback)();
            let delay = self.delay.clone();
            timeout(self, delay);
        }
    }

    timeout(Interval { callback: callback, delay: delay.clone() }, delay);
}

// Starts the event loop on this thread.
pub fn run() {
    EVENT_LOOP.with(move |event| event.borrow_mut().run().unwrap())
}

