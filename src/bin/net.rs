//! A TCP socket implementation of Hamelin.
#![feature(env, net, old_io)]
extern crate hamelin;
extern crate mio;

use std::collections::HashMap;
use std::env::args;
use std::old_io::IoErrorKind::TimedOut;
use hamelin::{BufferedAsyncStream, Hamelin, HamelinGuard};
use mio::{EventLoop, Handler, Interest, PollOpt, ReadHint, Token};
use mio::net::tcp::{TcpSocket, TcpStream, TcpListener};

const SERVER: Token = Token(0);

struct Client {
    stream: BufferedAsyncStream<TcpStream>,
    guard: HamelinGuard,
}

impl Client {
    fn new(sock: TcpStream, guard: HamelinGuard) -> Client {
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
    server: TcpListener,
    hamelin: Hamelin,
    token_index: usize,
    clients: HashMap<usize, Client>,
}

impl HamelinHandler {
    fn new(server: TcpListener, hamelin: Hamelin) -> HamelinHandler {
        HamelinHandler {
            server: server,
            hamelin: hamelin,
            token_index: 1,
            clients: HashMap::new(),
        }
    }

    fn accept(&mut self, eloop: &mut EventLoop<(), ()>) {
        if let Ok((client, _)) = self.server.accept() {
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
        if let Some(ref mut client) = self.clients.get_mut(&token) {
            let res = client.write();
            res
        } else {
            Err(())
        }
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
    let addr = format!("{}:{}", if args[0] == "localhost" { "127.0.0.1" } else { &*args[0] }, 
                       args[1]).parse().unwrap();
    let socket = TcpSocket::v4().unwrap();
    socket.bind(&addr).unwrap();
    let server = socket.listen(256).unwrap();
    println!("Server bound on {:?}.", addr);
    let mut eloop = EventLoop::<(), ()>::new().unwrap();
    eloop.register(&server, SERVER).unwrap();
    let mut handler = HamelinHandler::new(server, hamelin);
    eloop.run(&mut handler).ok().expect("Failed to execute event loop.");
}
