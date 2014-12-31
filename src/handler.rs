use mio::{IoDesc, IoHandle, event};

pub trait Handler: Send {
    fn readable(&mut self, hint: event::ReadHint) -> bool;
    fn writable(&mut self) -> bool;
    fn desc(&self) -> &IoDesc;

    fn interest(&self) -> Option<event::Interest> { None }
    fn opt(&self) -> Option<event::PollOpt> { None }
}

pub struct ClosureHandler<I, R, W> {
    pub io: I,
    pub read: R,
    pub write: W,
    pub interest: Option<event::Interest>,
    pub opt: Option<event::PollOpt>
}

impl<I, R, W> Handler for ClosureHandler<I, R, W>
where I: Send + IoHandle,
      R: Send + FnMut(&mut I, event::ReadHint) -> bool,
      W: Send + FnMut(&mut I) -> bool {
    fn readable(&mut self, hint: event::ReadHint) -> bool {
        (self.read)(&mut self.io, hint)
    }

    fn writable(&mut self) -> bool {
        (self.write)(&mut self.io)
    }

    fn desc(&self) -> &IoDesc {
        self.io.desc()
    }

    fn interest(&self) -> Option<event::Interest> {
        self.interest
    }

    fn opt(&self) -> Option<event::PollOpt> {
        self.opt
    }
}

