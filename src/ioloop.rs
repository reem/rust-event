use std::time::duration::Duration;
use std::thunk::Invoke;

use mio::util::Slab;
use mio::{EventLoop, EventLoopSender, Token, IoHandle, IoDesc, event};
use mio::Handler as MioHandler;

use {EventResult, EventError, Handler};

type MioEventLoop = EventLoop<Box<Invoke + 'static>, Registration>;

const MAX_LISTENERS: uint = 64 * 1024;

pub struct IoLoop {
    events: MioEventLoop
}

impl IoLoop {
    pub fn new() -> IoLoop {
        IoLoop {
            events: EventLoop::new().ok().expect("Failed to create mio event loop.")
        }
    }

    pub fn run(&mut self) -> EventResult<()> {
        match self.events.run(IoHandler::new()) {
            Ok(..) => Ok(()),
            Err(err) => {
                Err(EventError::MioError(err.error))
            }
        }
    }

    pub fn channel(&self) -> IoLoopSender {
        IoLoopSender { events: self.events.channel() }
    }

}

pub struct IoLoopSender {
    events: EventLoopSender<Registration>
}

impl IoLoopSender {
    pub fn send(&self, reg: Registration) {
        let _ = self.events.send(reg);
    }
}

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
        Registration::Timeout(box move |:_| { callback() }, timeout)
    }

    pub fn next<F: FnOnce() + 'static>(callback: F) -> Registration {
        Registration::Next(box move |:_| { callback() })
    }
}

struct Desc<'a> {
    desc: &'a IoDesc
}

impl<'a> Desc<'a> {
    fn new(desc: &'a IoDesc) -> Desc<'a> {
        Desc { desc: desc }
    }
}

impl<'a> IoHandle for Desc<'a> {
    fn desc(&self) -> &IoDesc { self.desc }
}

struct IoHandler {
    slab: Slab<Box<Handler>>,
}

impl IoHandler {
    fn new() -> IoHandler {
        IoHandler {
            slab: Slab::new(MAX_LISTENERS)
        }
    }

    fn register(&mut self, handler: Box<Handler>) -> Token {
        self.slab.insert(handler)
            .ok().expect("More than MAX_LISTENERS events registered.")
    }
}

impl MioHandler<Box<Invoke + 'static>, Registration> for IoHandler {
    fn readable(&mut self, events: &mut MioEventLoop, token: Token,
                hint: event::ReadHint) {
        // If this was deregistered during writable.
        if !self.slab.contains(token) { return }

        if self.slab[token].readable(hint) {
            let handler = &self.slab[token];
            let _ = events.reregister(
                &Desc::new(handler.desc()),
                token,
                handler.interest().unwrap_or(event::READABLE),
                handler.opt().unwrap_or(event::LEVEL)
            );
        } else {
            let _ = events.deregister(&Desc::new(self.slab[token].desc())).unwrap();
            self.slab.remove(token);
        }
    }

    fn writable(&mut self, events: &mut MioEventLoop, token: Token) {
        // If this was deregistered during readable.
        if !self.slab.contains(token) { return }

        if self.slab[token].writable() {
            let handler = &self.slab[token];
            let _ = events.reregister(
                &Desc::new(handler.desc()),
                token,
                handler.interest().unwrap_or(event::READABLE),
                handler.opt().unwrap_or(event::LEVEL)
            );
        } else {
            let _ = events.deregister(&Desc::new(self.slab[token].desc())).unwrap();
            self.slab.remove(token);
        }
    }

    fn notify(&mut self, events: &mut MioEventLoop, reg: Registration) {
        match reg {
            Registration::Handler(handler) => {
                let token = self.register(handler);
                let handler = &mut self.slab[token];

                let _ = events.register_opt(
                    &Desc::new(handler.desc()),
                    token,
                    handler.interest().unwrap_or(event::READABLE),
                    handler.opt().unwrap_or(event::LEVEL)
                );
            },

            Registration::Timeout(handler, timeout) => {
                let _ = events.timeout(handler, timeout);
            },

            Registration::Next(thunk) => { thunk.invoke(()) }
        }
    }

    fn timeout(&mut self, _: &mut MioEventLoop, thunk: Box<Invoke + 'static>) {
        thunk.invoke(())
    }
}

