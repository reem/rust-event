#![feature(unboxed_closures, phase)]

extern crate event;
extern crate mio;

#[phase(plugin, link)]
extern crate log;

use event::{run, register, ClosureHandler};

use mio::net::SockAddr;
use mio::net::tcp::{TcpSocket, TcpAcceptor};
use mio::{IoReader, IoWriter, IoAcceptor};
use mio::event as evt;

const RESPONSE: &'static str = "HTTP/1.1 200 OK\r
Content-Length: 14\r
\r
Hello World\r
\r";

fn read_socket(sock: &mut TcpSocket, _: evt::ReadHint) -> bool {
    info!("Reading from a socket.");
    match sock.read_slice(&mut [0; 1024]) {
        Ok(_) => { true }
        Err(ref e) if e.is_eof() => false,
        Err(e) => {
            error!("Error reading: {}", e);
            false
        }
    }
}

fn write_socket(sock: &mut TcpSocket) -> bool {
    info!("Writing to a socket.");
    match sock.write_slice(RESPONSE.as_bytes()) {
        Ok(_) => { true }
        Err(ref e) if e.is_eof() => false,
        Err(e) => {
            error!("Error writing: {}", e);
            false
        }
    }
}

fn accept(acceptor: &mut TcpAcceptor, _: evt::ReadHint) -> bool {
    info!("Accepting a connection.");
    let sock = acceptor.accept().unwrap();

    if !sock.would_block() {
        register(ClosureHandler {
            io: sock.unwrap(),
            read: read_socket,
            write: write_socket,
            interest: Some(evt::READABLE | evt::WRITABLE),
            opt: Some(evt::PollOpt::edge())
        });
    }

    true
}

fn main() {
    let addr = SockAddr::parse("127.0.0.1:3000")
        .expect("could not parse InetAddr");

    // Open socket
    let srv = TcpSocket::v4().unwrap()
        .bind(&addr).unwrap()
        .listen(256u).unwrap();

    register(ClosureHandler {
        io: srv,
        read: accept,
        write: move |&mut: _: &mut TcpAcceptor| true,
        interest: Some(evt::READABLE),
        opt: Some(evt::PollOpt::edge())
    });
    run();
}

