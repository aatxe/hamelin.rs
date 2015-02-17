//! An IRC implementation of Hamelin.
#![cfg_attr(feature = "irc", feature(core, env, std_misc))]
#[cfg(feature = "irc")] extern crate hamelin;
#[cfg(feature = "irc")] extern crate irc;

#[cfg(feature = "irc")] use std::borrow::ToOwned;
#[cfg(feature = "irc")] use std::collections::HashMap;
#[cfg(feature = "irc")] use std::default::Default;
#[cfg(feature = "irc")] use std::env::args;
#[cfg(feature = "irc")] use std::sync::{Arc, Mutex};
#[cfg(feature = "irc")] use std::thread::Thread;
#[cfg(feature = "irc")] use hamelin::{Hamelin, HamelinGuard};
#[cfg(feature = "irc")] use irc::client::data::{Command, Config};
#[cfg(feature = "irc")] use irc::client::data::Command::PRIVMSG;
#[cfg(feature = "irc")] use irc::client::server::{IrcServer, Server};
#[cfg(feature = "irc")] use irc::client::server::utils::Wrapper;

#[cfg(feature = "irc")]
fn main() {
    let args: Vec<_> = args().skip(1).collect();
    if args.len() < 4 {
        println!("Usage: hamelin host port nickname [-ssl] command [args]");
        return;
    }
    let use_ssl = args[3] == "-ssl";
    let hamelin = Arc::new(Hamelin::new(
        if use_ssl { &args[4] } else { &args[3] }, if use_ssl && args.len() > 5 {
            Some(&args[5..])
        } else if args.len() > 4 {
            Some(&args[4..])
        } else {
            None
        }
    ));    
    let config = Config {
        nickname: Some(args[2].to_owned()),
        server: Some(args[0].to_owned()),
        port: args[1].parse().ok(),
        channels: Some(vec!["#pdgn".to_owned()]),
        use_ssl: if use_ssl { Some(true) } else { None },
        .. Default::default()
    };
    let cache: Arc<Mutex<HashMap<String, HamelinGuard>>> = Arc::new(Mutex::new(HashMap::new()));
    loop {
        let irc_server = Arc::new(IrcServer::from_config(config.clone())
                                  .ok().expect("Failed to connect to IRC server."));
        let server_ref = irc_server.clone();
        let cache_ref = cache.clone();
        Thread::spawn(move || { 
            let server = Wrapper::new(&*server_ref);
            loop {
                for (resp, guard) in cache_ref.lock().unwrap().iter_mut() {
                    if let Ok(line) = guard.read_line() {
                        server.send_privmsg(&resp, &line).unwrap();
                    }
                }
            }
        });
        let server = Wrapper::new(&*irc_server);
        server.identify().unwrap();
        for message in server.iter() {
            match message {
                Ok(message) => {
                    print!("{}", message.into_string());
                    if let Ok(PRIVMSG(chan, msg)) = Command::from_message(&message) {
                        let mut cache = cache.lock().unwrap();
                        let resp = if chan.starts_with("#") { 
                            chan 
                        } else { 
                            message.get_source_nickname().unwrap_or(chan)
                        }.to_owned();
                        if !cache.contains_key(&resp) {
                            cache.insert(resp.clone(), hamelin.spawn().unwrap());
                        }
                        let _ = cache[resp].write_line(msg);
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
