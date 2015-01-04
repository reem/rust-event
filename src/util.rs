use mio::{IoDesc, IoHandle};

pub struct Desc<'a> {
    desc: &'a IoDesc
}

impl<'a> Desc<'a> {
    pub fn new(desc: &'a IoDesc) -> Desc<'a> {
        Desc { desc: desc }
    }
}

impl<'a> IoHandle for Desc<'a> {
    fn desc(&self) -> &IoDesc { self.desc }
}

