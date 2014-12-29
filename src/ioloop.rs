use mio::util::Slab;
use mio::{EventLoop, EventLoopSender, Token, IoHandle, IoDesc, event};
use mio::Handler as MioHandler;

use {EventResult};

const MAX_LISTENERS: uint = 64 * 1024;

pub struct IoLoop {
    events: EventLoop<(), Registration>
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

pub struct Registration {
    handler: Box<Handler>
}

impl Registration {
    pub fn new(handler: Box<Handler>) -> Registration {
        Registration { handler: handler }
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
    slab: Slab<Box<Handler>>
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

impl MioHandler<(), Registration> for IoHandler {
    fn readable(&mut self, events: &mut EventLoop<(), Registration>, token: Token,
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

    fn writable(&mut self, events: &mut EventLoop<(), Registration>, token: Token) {
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

    fn notify(&mut self, events: &mut EventLoop<(), Registration>, reg: Registration) {
        let handler = reg.handler;
        let token = self.register(handler);
        let handler = &mut self.slab[token];

        let _ = events.register_opt(
            &Desc::new(handler.desc()),
            token,
            handler.interest().unwrap_or(event::READABLE),
            handler.opt().unwrap_or(event::LEVEL)
        );
    }
}

