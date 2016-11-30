use mio::unix::EventedFd;
use codec::RpcCodec;
use std::cell::RefCell;
use std::rc::Rc;
use tokio_core::io::{Framed, Io};
use tokio_core::reactor::{Handle, PollEvented};
use std::process::{ChildStdout, ChildStdin};
use error::Result;
use std::result::Result as StdResult;
use std::io;
use std::os::unix::io::AsRawFd;
use std::process::Child;

pub struct AsyncChildIo {
    stdin: ChildStdin,
    stdout: ChildStdout,
}

impl AsyncChildIo {
    pub fn new(stdin: ChildStdin, stdout: ChildStdout) -> Self {
        AsyncChildIo { stdin, stdout }
    }

    pub fn into_lsio(self) -> LanguageServerIo {
        Rc::new(RefCell::new(self.framed(RpcCodec)))
    }
}

impl Io for AsyncChildIo { }

impl io::Read for AsyncChildIo {
    fn read(&mut self, buf: &mut [u8]) -> StdResult<usize, io::Error> {
        self.stdout.read(buf)
    }
}

impl io::Write for AsyncChildIo {
    fn write(&mut self, buf: &[u8]) -> StdResult<usize, io::Error> {
        self.stdin.write(buf)
    }

    fn flush(&mut self) -> StdResult<(), io::Error> {
        self.stdin.flush()
    }
}

pub type LanguageServerIo = Rc<RefCell<Framed<AsyncChildIo, RpcCodec>>>;
