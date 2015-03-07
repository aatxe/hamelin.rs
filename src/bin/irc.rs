//! An IRC implementation of Hamelin.
#[cfg(feature = "irc")] extern crate hamelin;
#[cfg(feature = "irc")] extern crate irc;

#[cfg(feature = "irc")] use std::borrow::ToOwned;
#[cfg(feature = "irc")] use std::collections::HashMap;
#[cfg(feature = "irc")] use std::env::args;
#[cfg(feature = "irc")] use std::sync::{Arc, Mutex};
#[cfg(feature = "irc")] use std::thread::spawn;
#[cfg(feature = "irc")] use hamelin::{Hamelin, HamelinGuard};
#[cfg(feature = "irc")] use irc::client::data::Command::PRIVMSG;
#[cfg(feature = "irc")] use irc::client::prelude::*;

#[cfg(feature = "irc")]
fn main() {
    let args: Vec<_> = args().skip(1).collect();
    if args.len() < 2 {
        println!("Usage: hamelin config command [args]");
        return;
    }
    let hamelin = Arc::new(Hamelin::new(&args[1], if args.len() > 2 { 
        Some(&args[2..]) 
    } else {
        None
    }));   
    let cache: Arc<Mutex<HashMap<String, HamelinGuard>>> = Arc::new(Mutex::new(HashMap::new()));
    loop {
        let server = Arc::new(IrcServer::new(&args[0])
                                  .ok().expect("Failed to connect to IRC server."));
        let tserver = server.clone();
        let tcache = cache.clone();
        spawn(move || { 
            loop {
                for (resp, guard) in tcache.lock().unwrap().iter_mut() {
                    if let Ok(line) = guard.read_line() {
                        tserver.send_privmsg(&resp, &line).unwrap();
                    }
                }
            }
        });
        server.identify().unwrap();
        for message in server.iter() {
            match message {
                Ok(message) => {
                    print!("{}", message.into_string());
                    if let Ok(PRIVMSG(chan, msg)) = Command::from_message(&message) {
                        let mut cache = cache.lock().unwrap();
                        let resp = if chan.starts_with("#") { 
                            &chan[..]
                        } else { 
                            message.get_source_nickname().unwrap_or(&chan)
                        }.to_owned();
                        if !cache.contains_key(&resp) {
                            let client_str = format!("{}://{}:{}/{}", if server.config().use_ssl() {
                                "ircs" } else { "irc" }, server.config().server(), 
                                server.config().port(), resp);
                            let env = vec![
                                ("H-TYPE", "HAMELIN.RS-IRC-0.1"),
                                ("H-CLIENT", &client_str)
                            ];
                            cache.insert(resp.clone(), hamelin.spawn_with_env(&env).unwrap());
                        }
                        let _ = cache[resp].write_line(&msg);
                    }
                },
                Err(e) => {
                    println!("Reconnecting because {}", e);
                    break
                }
            }
        }
    }
}

#[cfg(not(feature = "irc"))]
fn main() {
    println!("IRC daemon must be compiled with irc.");
}
