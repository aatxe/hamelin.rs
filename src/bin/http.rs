//! An HTTP implementation of Hamelin supporting GET and POST.
#![cfg_attr(feature = "hyper", feature(std_misc, thread_sleep))]
#[cfg(feature = "hyper")] extern crate hamelin;
#[cfg(feature = "hyper")] extern crate hyper;

#[cfg(feature = "hyper")] use std::borrow::ToOwned;
#[cfg(feature = "hyper")] use std::env::args;
#[cfg(feature = "hyper")] use std::io::prelude::*;
#[cfg(feature = "hyper")] use std::net::Ipv4Addr;
#[cfg(feature = "hyper")] use std::thread::sleep;
#[cfg(feature = "hyper")] use std::time::duration::Duration;
#[cfg(feature = "hyper")] use hamelin::Hamelin;
#[cfg(feature = "hyper")] use hyper::{Get, Post};
#[cfg(feature = "hyper")] use hyper::server::{Handler, Request, Response, Server};
#[cfg(feature = "hyper")] use hyper::uri::RequestUri::AbsolutePath;


#[cfg(feature = "hyper")]
struct HamelinHandler(Hamelin);
    
#[cfg(feature = "hyper")]
impl Handler for HamelinHandler {
    fn handle(&self, mut req: Request, mut res: Response) {
        let path = match req.uri {
            AbsolutePath(ref path) => match &req.method {
                &Post => path.to_owned(),
                &Get => path.to_owned(),
                _ => {
                    *res.status_mut() = hyper::NotFound;
                    res.start().and_then(|res| res.end()).unwrap();
                    return;
                }
            },
            _ => {
                res.start().and_then(|res| res.end()).unwrap();
                return;
            }
        };
        let mut guard = self.0.spawn_with_env(&[("H-TYPE", "HAMELIN.RS-HTTP-POST-0.1"),
                                                ("H-CLIENT", &path),
                                                ("H-URI", &path)]).unwrap();
        let mut data = String::new();
        req.read_to_string(&mut data).unwrap();
        guard.write_line(&data).unwrap();
        guard.eof().unwrap();
        sleep(Duration::milliseconds(100));
        let mut res = res.start().unwrap();
        while let Ok(line) = guard.read_line() {
            res.write_all(&line.as_bytes()).unwrap();
            res.flush().unwrap();
        }
        res.end().unwrap();
    }
}

#[cfg(feature = "hyper")]
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
    let server = Server::http(HamelinHandler(hamelin));
    let _guard = server.listen(
        (args[0].parse::<Ipv4Addr>().ok().expect("Invalid IP address."), 
         args[1].parse::<u16>().ok().expect("Invalid port number."))
    ).unwrap();
    println!("Listening on http://{}:{}/", args[0], args[1]);
}

#[cfg(not(feature = "hyper"))]
fn main() {
    println!("HTTP daemon must be compiled with hyper.");
}
