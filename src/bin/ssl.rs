//! A TCP socket implementation of Hamelin.
#![feature(core, env, io, os, std_misc)]
extern crate hamelin;
#[cfg(feature = "openssl")] extern crate openssl;

use std::env::args;
use std::old_io::{Acceptor, BufferedStream, Listener, TcpListener};
use std::sync::Arc;
use std::thread::Thread;
use hamelin::Hamelin;
#[cfg(feature = "openssl")] use openssl::ssl::{SslContext, SslMethod, SslStream};

#[cfg(feature = "openssl")]
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
        let ctx = SslContext::new(SslMethod::Sslv23).unwrap();
        let stream = SslStream::new_server(&ctx, stream.unwrap()).unwrap();
        let hamelin = hamelin.clone();
        Thread::spawn(move || {
            let mut stream = BufferedStream::new(stream);
            let mut guard = hamelin.spawn().unwrap();
            loop {
                if let Ok(line) = stream.read_line() { 
                    let _ = guard.write_line(&line);
                }
                if let Ok(line) = guard.read_line() {
                    let _ = stream.write_line(&line);
                    let _ = stream.flush();
                }
            }
        });
    }
}

#[cfg(not(feature = "openssl"))]
fn main() {
    println!("SSL daemon must be compiled with OpenSSL.");
}
