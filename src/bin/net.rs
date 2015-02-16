//! A TCP socket implementation of Hamelin.
#![feature(env, io)]
extern crate hamelin;
extern crate mio;

use std::collections::HashMap;
use std::env::args;
use std::old_io::IoErrorKind::TimedOut;
use hamelin::{BufferedAsyncStream, Hamelin, HamelinGuard};
use mio::{EventLoop, Handler, IoAcceptor, NonBlock, Token};
use mio::event::{Interest, PollOpt, ReadHint};
use mio::net::{SockAddr};
use mio::net::tcp::{TcpSocket, TcpAcceptor};

const SERVER: Token = Token(0);

struct Client {
    stream: BufferedAsyncStream<TcpSocket>,
    guard: HamelinGuard,
}

impl Client {
    fn new(sock: TcpSocket, guard: HamelinGuard) -> Client {
        Client {
            stream: BufferedAsyncStream::new(sock),
            guard: guard
        }
    }

    fn wait(&mut self) -> Result<(), ()> {
        self.guard.wait().map_err(|_| ())
    }

    fn read(&mut self) -> Result<(), ()> {
        match self.stream.read_line() {
            Ok(line) => match self.guard.write_line(&line) {
                Ok(_) => Ok(()),
                Err(ref e) if e.kind == TimedOut => Ok(()),
                Err(_) => Err(()),
            },
            Err(ref e) if e.kind == TimedOut => Ok(()),
            Err(_) => Err(()),
        }    
    }

    fn write(&mut self) -> Result<(), ()> {
        match self.guard.read_line() {
            Ok(line) => match self.stream.write_line(&line) {
                Ok(_) => {
                    Ok(())
                },
                Err(ref e) if e.kind == TimedOut => Ok(()),
                Err(_) => Err(()),
            },
            Err(ref e) if e.kind == TimedOut => Ok(()),
            Err(_) => Err(()),
        }
    }
}

struct HamelinHandler {
    server: TcpAcceptor,
    hamelin: Hamelin,
    token_index: usize,
    clients: HashMap<usize, Client>,
}

impl HamelinHandler {
    fn new(server: TcpAcceptor, hamelin: Hamelin) -> HamelinHandler {
        HamelinHandler {
            server: server,
            hamelin: hamelin,
            token_index: 1,
            clients: HashMap::new(),
        }
    }

    fn accept(&mut self, eloop: &mut EventLoop<(), ()>) {
        if let NonBlock::Ready(client) = self.server.accept().unwrap() {
            let token = mio::Token(self.token_index);
            self.token_index += 1;
            eloop.register_opt(&client, token, Interest::all(), PollOpt::level())
                 .ok().expect("Failed to accept new client.");
            self.clients.insert(
                token.as_usize(), Client::new(client, self.hamelin.spawn().unwrap())
            );
            println!("Client connected.");
        }
    }

    fn read(&mut self, token: usize) -> Result<(), ()> {
        let client = &mut self.clients[token];
        let res = client.read();
        res
    }

    fn write(&mut self, token: usize) -> Result<(), ()> {
        let client = &mut self.clients[token];
        let res = client.write();
        res
    }
}

impl Handler<(), ()> for HamelinHandler {
    fn readable(&mut self, eloop: &mut EventLoop<(), ()>, token: Token, _: ReadHint) {
        match token {
            SERVER => self.accept(eloop),
            Token(x) => {
                if let Err(_) = self.read(x) {
                    eloop.deregister(&self.clients[x].stream.stream).unwrap();
                    let _ = self.clients[x].wait();
                    self.clients.remove(&x);
                }
            },
        }
    }

    fn writable(&mut self, _: &mut EventLoop<(), ()>, token: Token) {
        match token {
            SERVER => (),
            Token(x) => {
                let _ = self.write(x);
            }
        }
    }
}

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
    let mut eloop = EventLoop::<(), ()>::new().unwrap();
    eloop.register(&server, SERVER).unwrap();
    eloop.run(HamelinHandler::new(server, hamelin)).ok().expect("Failed to execute event loop.");
}
