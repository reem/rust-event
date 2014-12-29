#![feature(unboxed_closures, phase)]

extern crate event;
extern crate mio;

#[phase(plugin, link)]
extern crate log;

use event::{run, register, Handler};

use mio::net::SockAddr;
use mio::net::tcp::{TcpSocket, TcpAcceptor};
use mio::{IoHandle, IoReader, IoWriter, IoAcceptor, IoDesc};
use mio::event as evt;

const RESPONSE: &'static str = "HTTP/1.1 200 OK\r
Content-Length: 11\r
\r
Hello World\r
\r";

struct TcpAcceptHandler {
    acceptor: TcpAcceptor
}

struct TcpConnHandler {
    sock: TcpSocket,
    message: &'static [u8],
    readbuf: [u8, ..64 * 1024]
}

impl Handler for TcpAcceptHandler {
    fn readable(&mut self, _: evt::ReadHint) -> bool {
        info!("Accepting a new conncetion.");
        self.accept();
        true
    }

    // Can't happen, no-op
    fn writable(&mut self) -> bool { true }

    fn desc(&self) -> &IoDesc {
        info!("Registering an acceptor.");
        self.acceptor.desc()
    }
}

impl TcpAcceptHandler {
    fn accept(&mut self) {
        let sock = self.acceptor.accept().unwrap();

        if !sock.would_block() {
            let conn = TcpConnHandler {
                sock: sock.unwrap(),
                message: RESPONSE.as_bytes(),
                readbuf: [0, ..64 * 1024]
            };
            register(conn);
        }
    }
}

impl Handler for TcpConnHandler {
    fn readable(&mut self, _: evt::ReadHint) -> bool {
        info!("Reading from an existing tcp connection.");

        match self.sock.read_slice(&mut self.readbuf) {
            Ok(_) => { true }
           Err(ref e) if e.is_eof() => false,
            Err(e) => {
                error!("Error reading: {}", e);
                false
            }
        }
    }

    fn writable(&mut self) -> bool {
        info!("Writing to an existing tcp connection.");

        match self.sock.write_slice(self.message) {
           Ok(wrote) => {
               self.message = self.message.slice_from(wrote.unwrap());
               true
           },
           Err(ref e) if e.is_eof() => false,
           Err(e) => {
               error!("Error writing: {}", e);
               false
           }
        }
    }

    fn desc(&self) -> &IoDesc { self.sock.desc() }

    fn interest(&self) -> Option<evt::Interest> {
        Some(evt::READABLE | evt::WRITABLE)
    }

    fn opt(&self) -> Option<evt::PollOpt> {
        Some(evt::PollOpt::edge())
    }
}

fn main() {
    let addr = SockAddr::parse("127.0.0.1:3000")
        .expect("could not parse InetAddr");

    // Open socket
    let srv = TcpSocket::v4().unwrap()
        .bind(&addr).unwrap()
        .listen(256u).unwrap();

    register(TcpAcceptHandler { acceptor: srv });
    run();
}

