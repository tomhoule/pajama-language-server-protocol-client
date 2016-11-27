use std::process::{Child, ChildStdin, ChildStdout};
use mio::{Evented, Poll, Token, Ready, PollOpt};
use mio::unix::EventedFd;
use std::io::{Read, Write};
use std::io;
use std::os::unix::io::AsRawFd;
use error::{Result as CustomResult};
use codec::RpcCodec;
use tokio_core::io::{Io, Framed};
use tokio_core::reactor::{Handle, PollEvented};

pub type IoWrapper = Framed<PollEvented<LanguageServerIo>, RpcCodec>;

pub fn make_io_wrapper(child: Child, handle: Handle) -> CustomResult<IoWrapper> {
    Ok(PollEvented::new(LanguageServerIo::new(child), &handle)?.framed(RpcCodec))
}

pub struct LanguageServerIo {
    child: Child,
}

impl LanguageServerIo {
    pub fn new(child: Child) -> LanguageServerIo {
        LanguageServerIo {
            child,
        }
    }

    fn get_stdin(&mut self) -> Result<&mut ChildStdin, io::Error> {
        if let Some(ref mut stdin) = self.child.stdin {
            Ok(stdin)
        } else {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "Could not reach server stdin"))
        }
    }

    fn get_stdout(&mut self) -> Result<&mut ChildStdout, io::Error> {
        if let Some(ref mut stdout) = self.child.stdout {
            Ok(stdout)
        } else {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "Could not reach server stdout"))
        }
    }

    fn get_stdin_ref(&self) -> Result<&ChildStdin, io::Error> {
        if let Some(ref stdin) = self.child.stdin {
            Ok(stdin)
        } else {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "Could not reach server stdout"))
        }
    }

    fn get_stdout_ref(&self) -> Result<&ChildStdout, io::Error> {
        if let Some(ref stdout) = self.child.stdout {
            Ok(stdout)
        } else {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "Could not reach server stdout"))
        }
    }
}

impl Read for LanguageServerIo {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, io::Error> {
        let stdout = self.get_stdout()?;
        stdout.read(buf)
    }
}

impl Write for LanguageServerIo {
    fn write(&mut self, buf: &[u8]) -> Result<usize, io::Error> {
        let stdin = self.get_stdin()?;
        stdin.write(buf)
    }

    fn flush(&mut self) -> Result<(), io::Error> {
        let stdin = self.get_stdin()?;
        stdin.flush()
    }
}

impl Evented for LanguageServerIo {
    fn register(
        &self,
        poll: &Poll,
        token: Token,
        interest: Ready,
        opts: PollOpt) -> Result<(), io::Error> {
        let stdin = self.get_stdin_ref()?;
        let stdout = self.get_stdout_ref()?;
        poll.register(&EventedFd(&stdin.as_raw_fd()), token, interest, opts)?;
        poll.register(&EventedFd(&stdout.as_raw_fd()), token, interest, opts)
    }

    fn reregister(
        &self,
        poll: &Poll,
        token: Token,
        interest: Ready,
        opts: PollOpt) -> Result<(), io::Error> {
        let stdin = self.get_stdin_ref()?;
        let stdout = self.get_stdout_ref()?;
        poll.reregister(&EventedFd(&stdin.as_raw_fd()), token, interest, opts)?;
        poll.reregister(&EventedFd(&stdout.as_raw_fd()), token, interest, opts)
    }

    fn deregister(&self, poll: &Poll) -> Result<(), io::Error> {
        let stdin = self.get_stdin_ref()?;
        let stdout = self.get_stdout_ref()?;
        poll.deregister(&EventedFd(&stdout.as_raw_fd()))?;
        poll.deregister(&EventedFd(&stdin.as_raw_fd()))
    }
}

#[cfg(test)]
mod tests {
    use mio::{Evented, Poll, PollOpt, Token, Ready};
    use super::LanguageServerIo;
    use std::process::{Command, Stdio};

    #[test]
    fn test_poll() {
        let child  = Command::new("/bin/sh")
            .arg("-c")
            .arg("echo mmmm")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let lsio = LanguageServerIo::new(child);
        let poll = Poll::new().unwrap();
        lsio.register(&poll, Token(600), Ready::all(), PollOpt::edge()).unwrap();
    }
}
