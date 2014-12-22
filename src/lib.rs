#![feature(macro_rules)]
#![deny(missing_docs, warnings)]

//! A multi-threaded Event-Loop for Rust.

use std::thunk::Thunk;
use std::{mem, ptr};

pub static EVENT_LOOP: EventLoop = EventLoop;

pub struct EventLoop;

impl Deref<EventLoopInner> for EventLoop {
    fn deref(&self) -> &EventLoopInner {
        use std::sync::{Once, ONCE_INIT};
        use std::mem;

        static mut DATA: *const EventLoop = 0 as *const _;
        static mut INIT: Once = ONCE_INIT;

        unsafe {
            INIT.doit(|:| DATA = mem::transmute(box EventLoopInner::new()));
            &*DATA
        }
    }
}

#[deriving(Default)]
struct EventLoopInner {
    timeouts: Timeouts,
    queued: Queued,
    io_events: MioEventLoop,
}

impl EventLoopInner {
    fn new() -> EventLoopInner {
        Default::default()
    }

    fn poll(&self) -> bool {
        self.queued.poll() || self.timeouts.poll() || self.io_events.poll()
    }
}

struct Timeouts {
    timeouts: [Option<Timeout>, ..1024]
}

impl Default for Timeouts {
    fn default() -> Timeouts {
        Timeouts {
            timeouts: [None, ..1024]
        }
    }
}

struct Timeout {
    end: uint,
    callback: Thunk
}

struct Queued {
    queued: [Option<Thunk>, ..64]
}

impl Default for Queued {
    fn default() -> Queued {
        Queued {
            queued: [None, ..64]
        }
    }
}

struct MioEventLoop {
    loops: uint
}

