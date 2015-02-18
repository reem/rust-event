use mio::{IoDesc, IoHandle, ReadHint, Interest, PollOpt};

pub trait Handler: 'static {
    fn readable(&mut self, hint: ReadHint) -> bool;
    fn writable(&mut self) -> bool;
    fn desc(&self) -> &IoDesc;

    fn interest(&self) -> Option<Interest> { None }
    fn opt(&self) -> Option<PollOpt> { None }
}

pub struct ClosureHandler<I, R, W> {
    pub io: I,
    pub read: R,
    pub write: W,
    pub interest: Option<Interest>,
    pub opt: Option<PollOpt>
}

impl<I, R, W> Handler for ClosureHandler<I, R, W>
where I: Send + IoHandle,
      R: Send + FnMut(&mut I, ReadHint) -> bool,
      W: Send + FnMut(&mut I) -> bool {
    fn readable(&mut self, hint: ReadHint) -> bool {
        (self.read)(&mut self.io, hint)
    }

    fn writable(&mut self) -> bool {
        (self.write)(&mut self.io)
    }

    fn desc(&self) -> &IoDesc {
        self.io.desc()
    }

    fn interest(&self) -> Option<Interest> {
        self.interest
    }

    fn opt(&self) -> Option<PollOpt> {
        self.opt
    }
}

