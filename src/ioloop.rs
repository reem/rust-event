use std::thunk::Invoke;
use std::time::duration::Duration;

use mio::util::Slab;
use mio::{EventLoop, Token, IoDesc, event};
use mio::Handler as MioHandler;
use util::Desc;

use mutscoped::MutScoped;

use {EventResult, EventError, Handler, HANDLER};

type MioEventLoop = EventLoop<Callback, Callback>;

pub type Callback = IsSend<Box<Invoke + 'static>>;

struct IsSend<T>(T);

unsafe impl<T: 'static> Send for IsSend<T> {}

const MAX_LISTENERS: usize = 64 * 1024;

pub struct IoLoop {
    events: MioEventLoop,
    handler: Option<IoHandler>
}

impl IoLoop {
    pub fn new() -> IoLoop {
        IoLoop {
            events: EventLoop::new().ok().expect("Failed to create mio event loop."),
            handler: Some(IoHandler::new())
        }
    }

    pub fn run(&mut self) -> EventResult<()> {
        match self.events.run(self.handler.take().unwrap()) {
            Ok(..) => Ok(()),
            Err(err) => {
                Err(EventError::MioError(err.error))
            }
        }
    }

    pub fn register<H: Handler>(&mut self, handler: H) {
        match self.handler {
            Some(ref mut iohandler) => {
                iohandler.register(&mut self.events, Box::new(handler));
            },

            None => HANDLER.with(move |iohandler| {
                unsafe { iohandler.borrow_mut(move |iohandler| {
                    iohandler.register(&mut self.events, Box::new(handler));
                }) }
            })
        }
    }


    pub fn timeout<F>(&mut self, callback: F, timeout: Duration)
    where F: FnOnce() + 'static {
        let _ = self.events.timeout(IsSend(Box::new(move |()| callback())), timeout);
    }

    pub fn next<F>(&mut self, callback: F) where F: FnOnce() + 'static {
        let _ = self.events.channel().send(IsSend(Box::new(move |()| callback())));
    }

    pub fn shutdown(&mut self) {
        self.events.shutdown()
    }
}

pub struct IoHandler {
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

impl MioHandler<Callback, Callback> for IoHandler {
    fn readable(&mut self, events: &mut MioEventLoop, token: Token,
                hint: event::ReadHint) {
        HANDLER.set(&MutScoped::new(self), || {
            if !self.slab.contains(token) { return }

            if self.slab[token].readable(hint) {
                self.reregister(events, token);
            } else {
                self.deregister(events, token);
            }
        })
    }

    fn writable(&mut self, events: &mut MioEventLoop, token: Token) {
        HANDLER.set(&MutScoped::new(self), || {
            if !self.slab.contains(token) { return }

            if self.slab[token].writable() {
                self.reregister(events, token);
            } else {
                self.deregister(events, token);
            }
        })
    }

    fn notify(&mut self, _: &mut MioEventLoop, thunk: IsSend<Box<Invoke + 'static>>) {
        thunk.0.invoke(())
    }

    fn timeout(&mut self, _: &mut MioEventLoop, thunk: IsSend<Box<Invoke + 'static>>) {
        thunk.0.invoke(())
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

