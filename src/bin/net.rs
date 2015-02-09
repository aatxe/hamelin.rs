//! A TCP socket implementation of Hamelin.
#![feature(core, env, io, os, std_misc)]
extern crate hamelin;

use std::env::args;
use std::old_io::{Acceptor, BufferedStream, Listener, TcpListener};
use std::old_io::IoErrorKind::TimedOut;
use std::sync::Arc;
use std::thread::Thread;
use hamelin::Hamelin;

fn main() {
    let args: Vec<_> = args().skip(1).map(|s| s.into_string().unwrap()).collect();
    if args.len() < 3 {
        println!("Usage: hamelin host port command [args]");
        return;
    }
    let hamelin = Arc::new(Hamelin::new(&args[2], if args.len() > 3 {
        Some(&args[3..])
    } else { 
        None 
    }));
    let listener = TcpListener::bind(&format!("{}:{}", args[0], args[1])[]);
    let mut acceptor = listener.listen();
    for stream in acceptor.incoming() {
        let mut stream = stream.unwrap();
        stream.set_timeout(Some(1));
        let hamelin = hamelin.clone();
        Thread::spawn(move || {
            let mut stream = BufferedStream::new(stream);
            let mut guard = hamelin.spawn().unwrap();
            loop {
                if let Ok(line) = guard.read_line() {
                    let _ = stream.write_line(&line);
                    let _ = stream.flush();
                }
                match stream.read_line() {
                    Ok(line) => {
                        let _ = guard.write_line(&line);
                    },
                    Err(ref e) if e.kind != TimedOut => {
                        let _ = guard.kill();
                        break;
                    },
                    _ => ()
                }
            }
        });
    }
}
