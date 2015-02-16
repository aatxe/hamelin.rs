//! A TCP socket implementation of Hamelin.
#![feature(env, io, std_misc)]
extern crate hamelin;
extern crate mio;

use std::env::args;
use std::old_io::IoErrorKind::TimedOut;
use std::thread::Thread;
use hamelin::{BufferedAsyncStream, Hamelin};
use mio::{event, EventLoop, IoAcceptor, Handler, Token};
use mio::net::{SockAddr};
use mio::net::tcp::{TcpSocket, TcpAcceptor};

const SERVER: Token = Token(0);

fn main() {
    let args: Vec<_> = args().skip(1).collect();
    if args.len() < 3 {
        println!("Usage: hamelin host port command [args]");
        return;
    }
    let hamelin = Hamelin::new(&args[2], if args.len() > 3 {
        Some(&args[3..])
    } else { 
        None 
    });
    let addr = SockAddr::parse(&format!("{}:{}", if args[0] == "localhost" { "127.0.0.1" } else 
                                        { &args[0][] }, args[1])).unwrap();
    let server = TcpSocket::v4().unwrap()
                    .bind(&addr).unwrap()
                    .listen(256).unwrap();
    println!("Server bound on {:?}.", addr);
    let mut event_loop = EventLoop::<(), ()>::new().unwrap();
    event_loop.register(&server, SERVER).unwrap();
    let _ = event_loop.run(HamelinHandler { server: server, hamelin: hamelin });
}

struct HamelinHandler {
    server: TcpAcceptor,
    hamelin: Hamelin,
}

impl Handler<(), ()> for HamelinHandler {
    fn readable(&mut self, _: &mut EventLoop<(), ()>, token: Token, _: event::ReadHint) {
        match token {
            SERVER => {
                let mut bufstream = BufferedAsyncStream::new(self.server.accept().unwrap().unwrap());
                let mut guard = self.hamelin.spawn().unwrap();
                Thread::spawn(move || {                
                    loop {
                        if let Ok(line) = guard.read_line() {
                            println!("Writing: {}", line);
                            let _ = bufstream.write_line(&line);
                        }
                        match bufstream.read_line() {
                            Ok(line) => {
                                println!("Read: {}", line);
                                let _ = guard.write_line(&line);
                            },
                            Err(ref e) if e.kind != TimedOut => {
                                break;
                            },
                            _ => ()
                        }  
                    }
                    guard.wait().unwrap();
                });
            }
            _ => panic!("unexpected token"),
        }
    }
}
