//! A Hamelin process-hosting backend.
#![feature(collections, core, io, std_misc)]

extern crate mio;

use std::borrow::ToOwned;
use std::old_io::{IoError, IoErrorKind, IoResult};
use std::old_io::process::{Command, Process};
use std::str::from_utf8;
use std::thread::Thread;
use mio::{IoReader, IoWriter, NonBlock, PipeReader, pipe};

pub struct Hamelin {
    process: String,
    args: Option<Vec<String>>,
}

impl Hamelin {
    pub fn new(process: &str, args: Option<&[String]>) -> Hamelin {
        Hamelin { 
            process: process.to_owned(), 
            args: args.map(|s| s.to_owned()),
        }
    }

    pub fn spawn(&self) -> IoResult<HamelinGuard> {
        let client_str = format!("{} ({:?})", self.process, self.args);
        let mut cmd = Command::new(&self.process);
        cmd.env_set_all(&[("H-VERSION", "hamelin.rs"),
                          ("H-TYPE", "HAMELIN.RS-NET-0.1"),
                          ("H-CLIENT", &client_str)]);
        if let Some(ref args) = self.args {
            cmd.args(args);
        }
        Ok(HamelinGuard::new(try!(cmd.spawn())))
    }
}

pub struct HamelinGuard {
    process: Process,
    alr: AsyncLineReader,
}

impl HamelinGuard {
    pub fn new(process: Process) -> HamelinGuard {
        let (read, write) = pipe().unwrap();
        let mut pipe = process.stdout.as_ref().map(|s| s.clone()).unwrap();
        Thread::spawn(move || {
            let mut buf = [0; 100];
            loop {
                let len = pipe.read(&mut buf).unwrap();
                write.write_slice(&mut buf[..len]).unwrap();
            }
        });
        HamelinGuard {
            process: process,
            alr: AsyncLineReader::new(read),
        }
    }

    pub fn write_line(&mut self, line: &str) -> IoResult<()> { 
        self.process.stdin.as_mut().map(|mut s| s.write_line(line)).unwrap()
    }

    pub fn read_line(&mut self) -> IoResult<String> {
        self.alr.read_line()
    }

    pub fn kill(&mut self) -> IoResult<()> {
        self.process.signal_kill()
    }
}

struct AsyncLineReader { read: PipeReader, buf: Vec<u8> }

impl AsyncLineReader {
    pub fn new(pr: PipeReader) -> AsyncLineReader {
        AsyncLineReader { read: pr, buf: Vec::new() }
    }

    pub fn read_line(&mut self) -> IoResult<String> {
        if let Some(pos) = self.buf.position_elem(&b'\n') {
            let mut rest = self.buf.split_off(pos);
            rest.remove(0);
            let result = from_utf8(&self.buf).unwrap().to_owned();
            self.buf = rest;
            return Ok(result);
        }
        let mut buf = [0; 100];
        match self.read.read_slice(&mut buf) {
            Ok(NonBlock::WouldBlock) => {
                return Err(IoError {
                    kind: IoErrorKind::TimedOut,
                    desc: "Reading would've blocked.",
                    detail: None,
                })
            },
            Ok(NonBlock::Ready(size)) => {  
                self.buf.push_all(&buf[..size]);
                match self.buf.position_elem(&b'\n') {
                    Some(pos) => {
                        let mut rest = self.buf.split_off(pos);
                        rest.remove(0);
                        let result = from_utf8(&self.buf).unwrap().to_owned();
                        self.buf = rest;
                        return Ok(result);
                    }
                    None => { 
                        return Err(IoError {
                            kind: IoErrorKind::TimedOut,
                            desc: "Reading would've blocked.",
                            detail: None,
                        })
                    }
                }
            }
            Err(e) => Err(IoError {
                kind: IoErrorKind::TimedOut,
                desc: "Reading would've blocked.",
                detail: Some(format!("{:?}", e)),
            })
        }
    }
}
