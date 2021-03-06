#![feature(unboxed_closures)]

extern crate event;
extern crate mio;

#[macro_use] extern crate log;

use event::{run, register, ClosureHandler};

use mio::net::SockAddr;
use mio::net::tcp::{TcpSocket, TcpAcceptor};
use mio::{IoWriter, IoAcceptor, PollOpt, Interest, ReadHint};

const RESPONSE: &'static str = "HTTP/1.1 200 OK\r
Content-Length: 14\r
\r
Hello World\r
\r";

fn accept(acceptor: &mut TcpAcceptor, _: ReadHint) -> bool {
    let sock = acceptor.accept().unwrap();

    if !sock.would_block() {
        register(ClosureHandler {
            io: sock.unwrap(),
            read: |_: &mut _, _| true,
            write: |sock: &mut TcpSocket| {
                loop {
                    match sock.write_slice(RESPONSE.as_bytes()) {
                        Ok(..) => {},
                        Err(..) => return false
                    }
                }
            },
            interest: Some(Interest::writable()),
            opt: Some(PollOpt::edge() | PollOpt::oneshot())
        }).unwrap();
    }

    true
}

fn main() {
    let addr = SockAddr::parse("127.0.0.1:3000").ok()
        .expect("could not parse InetAddr");

    // Open socket
    let srv = TcpSocket::v4().unwrap()
        .bind(&addr).unwrap()
        .listen(256).unwrap();

    register(ClosureHandler {
        io: srv,
        read: accept,
        write: move |_: &mut TcpAcceptor| true,
        interest: Some(Interest::readable()),
        opt: Some(PollOpt::edge() | PollOpt::oneshot())
    }).unwrap();

    run().unwrap();
}

