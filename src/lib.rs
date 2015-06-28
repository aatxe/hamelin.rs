//! A Hamelin process-hosting backend.
#![feature(collections, io)]

extern crate mio;
extern crate nix;

use std::borrow::ToOwned;
use std::ffi::OsStr;
use std::io::{Error, ErrorKind, Result};
use std::io::ErrorKind::{Other, TimedOut};
use std::os::unix::io::AsRawFd;
use std::process::{Child, Command};
use std::str::from_utf8;
use mio::{Buf, MutBuf, TryRead, TryWrite};
use mio::unix::{FromRawFd, PipeReader};
use nix::fcntl::{O_NONBLOCK, O_CLOEXEC, fcntl};
use nix::fcntl::FcntlArg::F_SETFL;
use nix::unistd::close;

/// A Hamelin daemon.
pub struct Hamelin {
    child: String,
    args: Option<Vec<String>>,
}

impl Hamelin {
    /// Constructs a Hamelin daemon from the child and optionally arguments.
    pub fn new(child: &str, args: Option<&[String]>) -> Hamelin {
        Hamelin {
            child: child.to_owned(),
            args: args.map(|s| s.to_owned()),
        }
    }

    /// Spawns a Hamelin server from the daemon.
    pub fn spawn(&self) -> Result<HamelinGuard> {
        let client_str = format!("{} ({:?})", self.child, self.args);
        let mut cmd = Command::new(&self.child);
        /*[("H-VERSION", "hamelin.rs"),
         ("H-TYPE", "HAMELIN.RS-GENERIC-0.1"),
         ("H-CLIENT", &client_str)])*/
        if let Some(ref args) = self.args {
            cmd.args(args);
        }
        Ok(HamelinGuard::new(try!(cmd.spawn())))
    }

    /// Spawns a Hamelin server from the daemon using the specified environment variables.
    pub fn spawn_with_env<K, V>(&self, env: &[(K, V)]) -> Result<HamelinGuard>
        where K: AsRef<OsStr>, V: AsRef<OsStr> {
        let mut cmd = Command::new(&self.child);
        //cmd.env_set_all(env);
        cmd.env("H-VERSION", "hamelin.rs");
        if let Some(ref args) = self.args {
            cmd.args(args);
        }
        Ok(HamelinGuard::new(try!(cmd.spawn())))
    }
}

/// A Hamelin server.
pub struct HamelinGuard {
    child: Child,
    alr: AsyncLineReader,
}

impl HamelinGuard {
    /// Creates a new Hamelin server from the specified child process.
    pub fn new(child: Child) -> HamelinGuard {
        let pipe = child.stdout.as_ref().map(|s| s.clone()).unwrap();
        let fd = pipe.as_raw_fd();
        fcntl(fd, F_SETFL(O_NONBLOCK | O_CLOEXEC)).unwrap();
        HamelinGuard {
            child: child,
            alr: AsyncLineReader::new(FromRawFd::from_raw_fd(fd)),
        }
    }

    /// Writes a line to the server's stdin.
    pub fn write_line(&mut self, line: &str) -> Result<()> {
        self.child.stdin.as_mut().map(|mut s| s.write_line(line)).unwrap()

    }

    /// Reads a line asynchronously from the server's stdout.
    pub fn read_line(&mut self) -> Result<String> {
        self.alr.read_line()
    }

    /// Closes the server's stdin.
    pub fn eof(&mut self) -> Result<()> {
        self.child.stdin.as_ref().map(|s| close(s.as_raw_fd())).unwrap().map_err(|_|
            Error::new(Other, "Failed to close stdin.")
        )
    }

    /// Awaits the completion of the server.
    pub fn wait(&mut self) -> Result<()> {
        self.child.wait().map(|_| ())
    }

    /// Sends a kill signal to the server.
    pub fn kill(&mut self) -> Result<()> {
        self.child.kill()
    }
}

struct AsyncLineReader { read: PipeReader, buf: Vec<u8> }

impl AsyncLineReader {
    pub fn new(pr: PipeReader) -> AsyncLineReader {
        AsyncLineReader { read: pr, buf: Vec::new() }
    }

    pub fn read_line(&mut self) -> Result<String> {
        if let Some(pos) = self.buf.iter().position(|x| x == &b'\n') {
            let mut rest = self.buf.split_off(pos);
            rest.remove(0);
            let result = from_utf8(&self.buf).unwrap().to_owned();
            self.buf = rest;
            return Ok(result);
        }
        let mut buf = [0; 100];
        match self.read.read_slice(&mut buf) {
            Ok(None) => {
                return Err(Error::new(TimedOut, "Reading would've blocked."))
            },
            Ok(Some(size)) => {
                self.buf.extend(buf[..size].iter().map(|x| *x));
                match self.buf.iter().position(|x| x == &b'\n') {
                    Some(pos) => {
                        let mut rest = self.buf.split_off(pos);
                        rest.remove(0);
                        let result = from_utf8(&self.buf).unwrap().to_owned();
                        self.buf = rest;
                        return Ok(result);
                    }
                    None => {
                        return Err(Error::new(TimedOut, "Reading would've blocked."))
                    }
                }
            }
            Err(_) => Err(Error::new(TimedOut, "Reading would've blocked."))
        }
    }
}

pub struct AsyncBufStream<T: Buf + MutBuf + TryRead + TryWrite> {
    pub stream: T,
    read_buf: Vec<u8>
}

impl<T: Buf + MutBuf + TryRead + TryWrite> AsyncBufStream<T> {
    pub fn new(stream: T) -> AsyncBufStream<T> {
        AsyncBufStream { stream: stream, read_buf: Vec::new() }
    }

    pub fn read_line(&mut self) -> Result<String> {
        if let Some(pos) = self.read_buf.iter().position(|x| x == &b'\n') {
            let mut rest = self.read_buf.split_off(pos);
            rest.remove(0);
            let result = from_utf8(&self.read_buf).unwrap().to_owned();
            self.read_buf = rest;
            return Ok(result);
        } else if let Some(_) = self.read_buf.iter().position(|x| x == &4) {
            return Err(Error::new(Other, "End of File reached."));
        }
        let mut buf = [0; 100];
        match self.stream.read_slice(&mut buf) {
            Ok(None) => Err(Error::new(TimedOut, "Reading would've blocked.")),
            Ok(Some(0)) => Err(Error::new(TimedOut, "Reading would've blocked.")),
            Ok(Some(size)) => {
                self.read_buf.extend(buf[..size].iter().map(|x| *x));
                match self.read_buf.iter().position(|x| x == &b'\n') {
                    Some(pos) => {
                        let mut rest = self.read_buf.split_off(pos);
                        rest.remove(0);
                        let result = from_utf8(&self.read_buf).unwrap().to_owned();
                        self.read_buf = rest;
                        return Ok(result);
                    }
                    None => {
                        return Err(Error::new(TimedOut, "Reading would've blocked."))
                    }
                }
            }
            Err(_) => Err(Error::new(TimedOut, "Reading would've blocked."))
        }
    }

    pub fn write_line(&mut self, s: &str) -> Result<()> {
        self.stream.write_slice(&s.as_bytes());
        self.stream.write_slice(b"\n");
        Ok(())
    }
}
