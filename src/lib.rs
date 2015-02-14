#![feature(unboxed_closures, core, std_misc, box_syntax)]
#![cfg_attr(test, deny(warnings))]
// #![deny(missing_docs)]

//! An Event-Loop for Rust.

extern crate mio;
extern crate core;

#[macro_use]
extern crate log;

use core::nonzero::NonZero;

use std::cell::RefCell;
use std::time::duration::Duration;
use ioloop::IoLoop;

pub use handler::{Handler, ClosureHandler};
pub use error::{EventResult, EventError};

use registration::Registration;

mod ioloop;
mod error;
mod handler;
mod registration;
mod util;

thread_local! {
    static EVENT_LOOP_LOCKED: RefCell<Box<IoLoop>> =
        RefCell::new(box IoLoop::new())
}

thread_local! {
    static EVENT_LOOP_FAST: RefCell<Option<NonZero<*mut IoLoop>>> =
        RefCell::new(None)
}

/// Register interest in an io descriptor.
pub fn register<H: Handler>(handler: H) {
    apply_events(move |events| events.register(handler));
}

/// Set a callback to be called soon after a certain timeout.
pub fn timeout<F: FnOnce() + 'static>(callback: F, timeout: Duration) {
    apply_events(move |events| events.timeout(callback, timeout));
}

/// File a callback to be called on the next run of the event loop.
pub fn next<F: FnOnce() + 'static>(callback: F) {
    apply_events(move |events| events.next(callback));
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
            timeout(self, delay);
        }
    }

    timeout(Interval { callback: callback, delay: delay.clone() }, delay);
}

/// Starts the event loop on this thread.
///
/// ## Panics
///
/// Panics if the event loop is already running on this thread. It should
/// not be called recursively (from within anything running off the event loop).
pub fn run() {
    EVENT_LOOP_LOCKED.with(move |event| {
        use std::cell::BorrowState::*;

        match event.borrow_state() {
            Writing => {
                warn!("Tried to start and event loop when the event loop for that \
                       thread was already running. Nooping.");
            },
            Unused => {
                debug!("Staring a thread local event loop.");
                let mut event_ref = event.borrow_mut();
                EVENT_LOOP_FAST.with(|cache| {
                    *cache.borrow_mut() = Some(unsafe { NonZero::new(&mut **event_ref) })
                });
                event_ref.run().unwrap()
            }
        }

    })
}

/// Shutdown the event loop on this thread, if it's running.
///
/// ## Warns
///
/// Warns if the event loop is not currently running on this thread.
pub fn shutdown() {
    EVENT_LOOP_LOCKED.with(move |event| {
        use std::cell::BorrowState::*;

        match event.borrow_state() {
            Reading | Writing => {
                getfast(move |event| {
                    debug!("Shutting down the event loop.");
                    event.shutdown()
                })
            },
            Unused => {
                warn!("Tried to shutdown the event loop when it wasn't running.")
            },

        }
    })
}

fn apply_events<A>(action: A)
where A: FnOnce(&mut IoLoop) {
    EVENT_LOOP_LOCKED.with(move |event| {
        use std::cell::BorrowState::*;

        match event.borrow_state() {
            Reading | Writing => {
                getfast(move |event| {
                    action(event)
                })
            },

            Unused => {
                action(&mut event.borrow_mut())
            }
        }
    })
}

fn getfast<T, F: for<'a> FnOnce(&'a mut IoLoop) -> T>(f: F) -> T {
    use std::mem;

    EVENT_LOOP_FAST.with(move |event| {
        f(event.borrow_mut().map(|ptr| {
            unsafe { mem::transmute::<*mut IoLoop, &mut IoLoop>(*ptr) }
        }).expect("Tried to get EVENT_LOOP_FAST before event loop was run."))
    })
}

