#![feature(unboxed_closures, core, std_misc, box_syntax)]
#![cfg_attr(test, deny(warnings))]
// #![deny(missing_docs)]

//! An Event-Loop for Rust.

extern crate mio;
extern crate core;

#[macro_use]
extern crate log;

use std::cell::RefCell;
use std::time::duration::Duration;

use ioloop::{IoLoop, IoHandler};
use mutscoped::MutScoped;

pub use mio::Timeout;
pub use handler::{Handler, ClosureHandler};
pub use error::{EventResult, EventError};

mod ioloop;
mod error;
mod handler;
mod util;
mod mutscoped;

thread_local! {
    static EVENT_LOOP_LOCKED: RefCell<Box<IoLoop>> =
        RefCell::new(box IoLoop::new())
}

scoped_thread_local!(static EVENT_LOOP_FAST: MutScoped<IoLoop>);
scoped_thread_local!(static HANDLER: MutScoped<IoHandler>);

/// Register interest in an io descriptor.
pub fn register<H: Handler>(handler: H) -> EventResult<()> {
    apply_events(move |events| events.register(handler))
}

/// Set a callback to be called soon after a certain timeout.
pub fn timeout<F>(callback: F, timeout: Duration) -> EventResult<Timeout>
where F: FnOnce() + 'static {
    apply_events(move |events| events.timeout(callback, timeout))
}

/// File a callback to be called on the next run of the event loop.
pub fn next<F: FnOnce() + 'static>(callback: F) -> Result<(), ()> {
    apply_events(move |events| events.next(callback))
}

pub fn interval<F: FnMut() + Send>(callback: F, delay: Duration) {
    struct Interval<F> {
        callback: F,
        delay: Duration
    }

    impl<F: FnMut() + Send()> FnOnce<()> for Interval<F> {
        type Output = ();

        extern "rust-call" fn call_once(mut self, _: ()) {
            (self.callback)();
            let delay = self.delay.clone();
            timeout(self, delay).unwrap();
        }
    }

    timeout(Interval { callback: callback, delay: delay.clone() }, delay).unwrap();
}

/// Starts the event loop on this thread.
///
/// ## Panics
///
/// Panics if the event loop is already running on this thread. It should
/// not be called recursively (from within anything running off the event loop).
pub fn run() -> Result<(), EventError> {
    if EVENT_LOOP_FAST.is_set() {
        warn!("Tried to start and event loop when the event loop for that \
               thread was already running. Nooping.");
        Ok(())
    } else {
        EVENT_LOOP_LOCKED.with(move |event| {
            let mut event_ref = event.borrow_mut();
            EVENT_LOOP_FAST.set(
                &MutScoped::new(&mut *event_ref),
                || { event_ref.run() }
            )
        })
    }
}

/// Shutdown the event loop on this thread, if it's running.
///
/// ## Warns
///
/// Warns if the event loop is not currently running on this thread.
pub fn shutdown() {
    if EVENT_LOOP_FAST.is_set() {
        EVENT_LOOP_FAST.with(|event| {
            debug!("Shutting down the event loop.");
            unsafe { event.borrow_mut(|event| event.shutdown()) }
        })
    } else {
        warn!("Tried to shutdown the event loop when it wasn't running.")
    }
}

fn apply_events<A, R>(action: A) -> R
where A: FnOnce(&mut IoLoop) -> R {
    if EVENT_LOOP_FAST.is_set() {
        EVENT_LOOP_FAST.with(move |event| unsafe { event.borrow_mut(action) })
    } else {
        EVENT_LOOP_LOCKED.with(move |event| action(&mut event.borrow_mut()))
    }
}

