//! An HTTP POST-based implementation of Hamelin.
#![cfg_attr(feature = "hyper", feature(env, old_io, std_misc))]
#[cfg(feature = "hyper")] extern crate hamelin;
#[cfg(feature = "hyper")] extern crate hyper;

#[cfg(feature = "hyper")] use std::borrow::ToOwned;
#[cfg(feature = "hyper")] use std::env::args;
#[cfg(feature = "hyper")] use std::old_io::timer::sleep;
#[cfg(feature = "hyper")] use std::time::duration::Duration;
#[cfg(feature = "hyper")] use hamelin::Hamelin;
#[cfg(feature = "hyper")] use hyper::Post;
#[cfg(feature = "hyper")] use hyper::server::{Handler, Request, Response, Server};
#[cfg(feature = "hyper")] use hyper::uri::RequestUri::AbsolutePath;


#[cfg(feature = "hyper")]
struct HamelinHandler(Hamelin);
    
#[cfg(feature = "hyper")]
impl Handler for HamelinHandler {
    fn handle(&self, mut req: Request, mut res: Response) {
        let &HamelinHandler(ref hamelin) = self;
        let path = match req.uri {
            AbsolutePath(ref path) => match &req.method {
                &Post => path.to_owned(),
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
        let mut guard = hamelin.spawn_with_env(&[("H-TYPE", "HAMELIN.RS-HTTP-POST-0.1"),
                                                 ("H-URI", &path)]).unwrap();
        guard.write_line(&req.read_to_string().unwrap()).unwrap();
        sleep(Duration::milliseconds(100));
        let mut res = res.start().unwrap();
        while let Ok(line) = guard.read_line() {
            res.write_line(&line).unwrap();
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
    let server = Server::http(args[0].parse().ok().expect("Invalid IP address."), 
                              args[1].parse().ok().expect("Invalid port number."));
    let _guard = server.listen(HamelinHandler(hamelin)).unwrap();
    println!("Listening on http://{}:{}/", args[0], args[1]);
}

#[cfg(not(feature = "hyper"))]
fn main() {
    println!("HTTP daemon must be compiled with hyper.");
}
