use std::time::duration::Duration;
use std::thunk::Thunk;

use mio::util::Slab;
use mio::{EventLoop, EventLoopSender, Token, IoHandle, IoDesc, event};
use mio::Handler as MioHandler;

use {EventResult};

type MioEventLoop = EventLoop<Thunk, Registration>;

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
        Ok(try!(self.events.run(IoHandler::new()).map(|_| ())))
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

pub trait Handler: Send {
    fn readable(&mut self, hint: event::ReadHint) -> bool;
    fn writable(&mut self) -> bool;
    fn desc(&self) -> &IoDesc;

    fn interest(&self) -> Option<event::Interest> { None }
    fn opt(&self) -> Option<event::PollOpt> { None }
}

pub enum Registration {
    Handler(Box<Handler>),
    Timeout(Thunk, Duration)
}

impl Registration {
    pub fn new(handler: Box<Handler>) -> Registration {
        Registration::Handler(handler)
    }

    pub fn timeout<F: FnOnce() + Send>(callback: F, timeout: Duration) -> Registration {
        Registration::Timeout(Thunk::new(callback), timeout)
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

impl MioHandler<Thunk, Registration> for IoHandler {
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
            }
        }
    }

    fn timeout(&mut self, _: &mut MioEventLoop, thunk: Thunk) {
        thunk.invoke(())
    }
}

