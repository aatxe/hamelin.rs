//! A Hamelin process-hosting backend.
#![feature(collections, core, io, std_misc)]

extern crate mio;

use std::borrow::ToOwned;
use std::old_io::{IoError, IoErrorKind, IoResult};
use std::old_io::process::{Command, Process};
use std::str::from_utf8;
use std::thread::Thread;
use mio::{IoReader, IoWriter, NonBlock, PipeReader, pipe};

/// A Hamelin daemon.
pub struct Hamelin {
    process: String,
    args: Option<Vec<String>>,
}

impl Hamelin {
    /// Constructs a Hamelin daemon from the process and optionally arguments.
    pub fn new(process: &str, args: Option<&[String]>) -> Hamelin {
        Hamelin { 
            process: process.to_owned(), 
            args: args.map(|s| s.to_owned()),
        }
    }

    /// Spawns a Hamelin server from the daemon.
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

/// A Hamelin server.
pub struct HamelinGuard {
    process: Process,
    alr: AsyncLineReader,
}

impl HamelinGuard {
    /// Creates a new Hamelin server from the specified process.
    pub fn new(process: Process) -> HamelinGuard {
        let (read, write) = pipe().unwrap();
        let mut pipe = process.stdout.as_ref().map(|s| s.clone()).unwrap();
        Thread::spawn(move || {
            let mut buf = [0; 100];
            loop {
                match pipe.read(&mut buf) {
                    Ok(len) => {
                        write.write_slice(&mut buf[..len]).unwrap();
                    },
                    Err(ref e) if e.kind != IoErrorKind::TimedOut => break,
                    _ => (),
                }
            }
        });
        HamelinGuard {
            process: process,
            alr: AsyncLineReader::new(read),
        }
    }

    /// Writes a line to the server's stdin.
    pub fn write_line(&mut self, line: &str) -> IoResult<()> { 
        self.process.stdin.as_mut().map(|mut s| s.write_line(line)).unwrap()
    }

    /// Reads a line asynchronously from the server's stdout.
    pub fn read_line(&mut self) -> IoResult<String> {
        self.alr.read_line()
    }

    /// Awaits the completion of the server.
    pub fn wait(&mut self) -> IoResult<()> {
        self.process.wait().map(|_| ())
    }

    /// Sends a kill signal to the server.
    pub fn kill(&mut self) -> IoResult<()> {
        try!(self.process.signal_exit());
        self.process.set_timeout(Some(1000));
        if let Ok(_) = self.process.wait() {
            return Ok(())
        }
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

pub struct BufferedAsyncStream<T: IoReader + IoWriter> {
    stream: T,
    read_buf: Vec<u8>
}

impl<T: IoReader + IoWriter> BufferedAsyncStream<T> {
    pub fn new(stream: T) -> BufferedAsyncStream<T> {
        BufferedAsyncStream { stream: stream, read_buf: Vec::new() }
    }

    pub fn read_line(&mut self) -> IoResult<String> {
        if let Some(pos) = self.read_buf.position_elem(&b'\n') {
            let mut rest = self.read_buf.split_off(pos);
            rest.remove(0);
            let result = from_utf8(&self.read_buf).unwrap().to_owned();
            self.read_buf = rest;
            return Ok(result);
        } else if let Some(_) = self.read_buf.position_elem(&4) {
            return Err(IoError {
                kind: IoErrorKind::EndOfFile,
                desc: "End of File reached.",
                detail: None,
            });
        }
        let mut buf = [0; 100];
        match self.stream.read_slice(&mut buf) {
            Ok(NonBlock::WouldBlock) => Err(IoError {
                kind: IoErrorKind::TimedOut,
                desc: "Reading would've blocked.",
                detail: None,
            }),
            Ok(NonBlock::Ready(size)) => {  
                self.read_buf.push_all(&buf[..size]);
                match self.read_buf.position_elem(&b'\n') {
                    Some(pos) => {
                        let mut rest = self.read_buf.split_off(pos);
                        rest.remove(0);
                        let result = from_utf8(&self.read_buf).unwrap().to_owned();
                        self.read_buf = rest;
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
            Err(e) if e.is_eof() => Err(IoError {
                kind: IoErrorKind::EndOfFile,
                desc: "End of File reached.",
                detail: Some(format!("{:?}", e)),
            }),
            Err(e) => Err(IoError {
                kind: IoErrorKind::TimedOut,
                desc: "Reading would've blocked.",
                detail: Some(format!("{:?}", e)),
            })
        }
    }

    pub fn write_line(&mut self, s: &str) -> IoResult<()> {
        match self.stream.write_slice(&s.as_bytes()) {
            Ok(NonBlock::WouldBlock) => Err(IoError {
                kind: IoErrorKind::TimedOut,
                desc: "Writing would've blocked.",
                detail: None,
            }),
            Ok(NonBlock::Ready(_)) => {  
                match self.stream.write_slice(&b"\n") {
                    Ok(NonBlock::WouldBlock) => Err(IoError {
                        kind: IoErrorKind::TimedOut,
                        desc: "Writing would've blocked.",
                        detail: None,
                    }),
                    Ok(_) => Ok(()),
                    Err(e) => Err(IoError {
                        kind: IoErrorKind::TimedOut,
                        desc: "Writing would've blocked.",
                        detail: Some(format!("{:?}", e)),
                    })
                }
            }
            Err(e) if e.is_eof() => Err(IoError {
                kind: IoErrorKind::EndOfFile,
                desc: "End of File reached.",
                detail: Some(format!("{:?}", e)),
            }),
            Err(e) => Err(IoError {
                kind: IoErrorKind::TimedOut,
                desc: "Writing would've blocked.",
                detail: Some(format!("{:?}", e)),
            })   
        }
    }
}
