#![allow(unstable)]
#![feature(unboxed_closures)]
#![cfg_attr(test, deny(warnings))]
// #![deny(missing_docs)]

//! An Event-Loop for Rust.

extern crate mio;

use std::cell::RefCell;
use std::time::duration::Duration;
use ioloop::{IoLoop, IoLoopSender};

pub use handler::{Handler, ClosureHandler};
pub use error::{EventResult, EventError};

use registration::Registration;

mod ioloop;
mod error;
mod handler;
mod registration;
mod util;

thread_local! {
    static EVENT_LOOP: RefCell<IoLoop> =
        RefCell::new(IoLoop::new())
}

thread_local! {
    static EVENT_LOOP_SENDER: IoLoopSender =
        EVENT_LOOP.with(move |event| event.borrow().channel())
}

pub fn register<H: Handler>(handler: H) {
    EVENT_LOOP_SENDER.with(move |events| {
        let _ = events.send(Registration::new(Box::new(handler)));
    });
}

pub fn timeout<F: FnOnce() + 'static>(callback: F, timeout: Duration) {
    EVENT_LOOP_SENDER.with(move |events| {
        let _ = events.send(Registration::timeout(callback, timeout));
    });
}

pub fn next<F: FnOnce() + 'static>(callback: F) {
    EVENT_LOOP_SENDER.with(move |events| {
        let _ = events.send(Registration::next(callback));
    });
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

