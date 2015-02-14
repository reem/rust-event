use std::thunk::Invoke;

use mio::util::Slab;
use mio::{EventLoop, EventLoopSender, Token, IoDesc, event};
use mio::Handler as MioHandler;

use registration::Registration;
use util::Desc;

use {EventResult, EventError, Handler};

type MioEventLoop = EventLoop<Box<Invoke + 'static>, Box<Handler>>;

const MAX_LISTENERS: usize = 64 * 1024;

pub struct IoLoop {
    events: MioEventLoop,
    handler: IoHandler
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

    pub fn register<H: Handler>(&mut self, handler: H) {
        self.handler.register(&mut self.events, Box::new(handler));
    }


    pub fn timeout<F>(&mut self, callback: F, timeout: Duration)
    where F: FnOnce() + 'static {
        let _ = self.events.timeout(Box::new(callback), timeout);
    }

    pub fn next<F>(&mut self, callback: F) where F: FnOnce() + 'static {
        self.events.channel().send(Box::new(callback))
    }

    pub fn shutdown(&mut self) {
        self.events.shutdown()
    }

    pub fn channel(&self) -> IoLoopSender {
        self.events.channel()
    }
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

    fn register(&mut self, events: &mut MioEventLoop, handler: Box<Handler>) {
        let token = self.slab.insert(handler)
            .ok().expect("More than MAX_LISTENERS events registered.");
        register(events, &mut *self.slab[token], token);
    }

    fn reregister(&mut self, events: &mut MioEventLoop, token: Token) {
        reregister(events, &mut *self.slab[token], token);
    }

    fn deregister(&mut self, events: &mut MioEventLoop, token: Token) {
        deregister(events, self.slab[token].desc());
        self.slab.remove(token);
    }
}

impl MioHandler<Box<Invoke + 'static>, Box<Handler>> for IoHandler {
    fn readable(&mut self, events: &mut MioEventLoop, token: Token,
                hint: event::ReadHint) {
        if !self.slab.contains(token) { return }

        if self.slab[token].readable(hint) {
            self.reregister(events, token);
        } else {
            self.deregister(events, token);
        }
    }

    fn writable(&mut self, events: &mut MioEventLoop, token: Token) {
        if !self.slab.contains(token) { return }

        if self.slab[token].writable() {
            self.reregister(events, token);
        } else {
            self.deregister(events, token);
        }
    }

    fn notify(&mut self, _: &mut MioEventLoop, thunk: Box<Invoke + 'static>) {
        thunk.invoke(())
    }

    fn timeout(&mut self, _: &mut MioEventLoop, thunk: Box<Invoke + 'static>) {
        thunk.invoke(())
    }
}

fn register(events: &mut MioEventLoop, handler: &mut Handler, token: Token) {
    let _ = events.register_opt(
        &Desc::new(handler.desc()),
        token,
        handler.interest().unwrap_or(event::READABLE),
        handler.opt().unwrap_or(event::LEVEL)
    );
}

fn reregister(events: &mut MioEventLoop, handler: &mut Handler, token: Token) {
    let _ = events.reregister(
        &Desc::new(handler.desc()),
        token,
        handler.interest().unwrap_or(event::READABLE),
        handler.opt().unwrap_or(event::LEVEL)
    );
}

fn deregister(events: &mut MioEventLoop, desc: &IoDesc) {
    let _ = events.deregister(&Desc::new(desc)).unwrap();
}

