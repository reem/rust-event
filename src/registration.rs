use handler::Handler;
use std::thunk::Invoke;
use std::time::duration::Duration;

pub enum Registration {
    Handler(Box<Handler>),
    Timeout(Box<Invoke + 'static>, Duration),
    Next(Box<Invoke + 'static>)
}

// This is an *incorrect* implementation, in the sense that
// sending a Registration can be incorrect. However, we
// only send Registration and its contents around a single
// thread so it is ok for us to fake Send to fool mio.
unsafe impl Send for Registration {}

impl Registration {
    pub fn new(handler: Box<Handler>) -> Registration {
        Registration::Handler(handler)
    }

    pub fn timeout<F: FnOnce() + 'static>(callback: F, timeout: Duration) -> Registration {
        Registration::Timeout(Box::new(move |:_| { callback() }), timeout)
    }

    pub fn next<F: FnOnce() + 'static>(callback: F) -> Registration {
        Registration::Next(Box::new(move |:_| { callback() }))
    }
}

