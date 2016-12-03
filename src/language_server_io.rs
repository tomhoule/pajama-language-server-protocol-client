use mio::unix::EventedFd;
use futures::{Async};
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
use mio;

struct Stdin(ChildStdin);
struct Stdout(ChildStdout);

impl io::Write for Stdin {
    fn write(&mut self, buf: &[u8]) -> StdResult<usize, io::Error> {
        self.0.write(buf)
    }

    fn flush(&mut self) -> StdResult<(), io::Error> {
        self.0.flush()
    }
}

impl io::Read for Stdout {
    fn read(&mut self, buf: &mut [u8]) -> StdResult<usize, io::Error> {
        self.0.read(buf)
    }
}


impl mio::Evented for Stdin {
    fn register(&self,
                poll: &mio::Poll,
                token: mio::Token,
                interest: mio::Ready,
                opts: mio::PollOpt) -> io::Result<()> {
        EventedFd(&self.0.as_raw_fd()).register(poll, token, interest, opts)
    }

    fn reregister(&self,
                  poll: &mio::Poll,
                  token: mio::Token,
                  interest: mio::Ready,
                  opts: mio::PollOpt) -> io::Result<()> {
        EventedFd(&self.0.as_raw_fd()).reregister(poll, token, interest, opts)
    }

    fn deregister(&self, poll: &mio::Poll) -> io::Result<()> {
        EventedFd(&self.0.as_raw_fd()).deregister(poll)
    }
}

impl mio::Evented for Stdout {
    fn register(&self,
                poll: &mio::Poll,
                token: mio::Token,
                interest: mio::Ready,
                opts: mio::PollOpt) -> io::Result<()> {
        EventedFd(&self.0.as_raw_fd()).register(poll, token, interest, opts)
    }

    fn reregister(&self,
                  poll: &mio::Poll,
                  token: mio::Token,
                  interest: mio::Ready,
                  opts: mio::PollOpt) -> io::Result<()> {
        EventedFd(&self.0.as_raw_fd()).reregister(poll, token, interest, opts)
    }

    fn deregister(&self, poll: &mio::Poll) -> io::Result<()> {
        EventedFd(&self.0.as_raw_fd()).deregister(poll)
    }
}


pub struct AsyncChildIo {
    stdin: PollEvented<Stdin>,
    stdout: PollEvented<Stdout>,
}

impl AsyncChildIo {
    pub fn new(child: Child, handle: &Handle) -> Result<Self> {
        let raw_stdin = Stdin(child.stdin.unwrap());
        let stdin = PollEvented::new(raw_stdin, handle)?;
        let raw_stdout = Stdout(child.stdout.unwrap());
        let stdout = PollEvented::new(raw_stdout, handle)?;
        Ok(AsyncChildIo {
            stdin,
            stdout,
        })
    }

    pub fn into_lsio(self) -> LanguageServerIo {
        Rc::new(RefCell::new(self.framed(RpcCodec)))
    }
}

impl Io for AsyncChildIo {
    fn poll_read(&mut self) -> Async<()> {
        self.stdout.poll_read()
    }

    fn poll_write(&mut self) -> Async<()> {
        self.stdin.poll_write()
    }
}

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

#[cfg(test)]
mod test {
    extern crate env_logger;

    use super::*;
    use futures::*;
    use tokio_core::io::*;
    use tokio_core::reactor::*;
    use std::process::*;
    use std::io::{Read, Write};
    use std::time::Duration;
    use futures::stream::*;
    use std::str;

    struct WritePoller {
        count: i32,
        inner: WriteHalf<AsyncChildIo>,
    }

    impl Stream for WritePoller {
        type Item = ();
        type Error = ();

        fn poll(&mut self) -> Poll<Option<()>, ()> {
            if self.count > 100 {
                debug!("write stream complete");
                return Ok(Async::Ready(None))
            }
            if let Async::Ready(()) = self.inner.poll_write() {
                let written = self.inner.write("lorem ipsum\n\n".as_bytes()).unwrap();
                debug!("write - wrote {:?} bytes", written);
                self.inner.flush();
                self.count +=1;
                return Ok(Async::Ready(Some(())));
            }
            Ok(Async::NotReady)
        }
    }

    struct ReadPoller {
        count: i32,
        data: Vec<u8>,
        inner: ReadHalf<AsyncChildIo>,
    }

    impl Stream for ReadPoller {
        type Item = ();
        type Error = ();

        fn poll(&mut self) -> Poll<Option<()>, ()> {
            if self.count > 100 {
                debug!("read stream completed ({:?})", self.data);
                return Ok(Async::Ready(None))
            }

            if let Async::Ready(()) = self.inner.poll_read() {
                let read_size = self.inner.read(&mut self.data).unwrap();
                debug!("read - read {:?} bytes", read_size);
                if read_size < self.data.len() {
                    return Ok(Async::Ready(None))
                }
                self.count += 1;
                return Ok(Async::Ready(Some(())));
            } else {
                debug!("read - stdout not ready");
                Ok(Async::NotReady)
            }
        }
    }

    #[test]
    fn stdout_can_be_read_from() {
        let mut child = Command::new("echo").arg("meh").stdout(Stdio::piped()).spawn().unwrap();
        let mut r = Stdout(child.stdout.take().unwrap());
        let mut buf = [b'0'; 10];
        r.read(&mut buf).unwrap();
        assert_eq!(str::from_utf8(&buf).unwrap(), "meh\n000000");
    }

    #[test]
    fn stdout_can_read_from_stdin() {
        let mut child = Command::new("cat")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let mut r = Stdout(child.stdout.take().unwrap());
        let mut w = Stdin(child.stdin.take().unwrap());

        let mut in_buf = Vec::new();
        let mut out_buf = vec![0u8; 100];

        for _ in 0..80 {
            in_buf.push(b'a')
        }

        w.write(in_buf.as_slice()).unwrap();
        w.flush().unwrap();

        assert_eq!(80, r.read(&mut out_buf).unwrap());

    }

    // #[test]
    fn async_child_io_does_not_hang() {
        drop(env_logger::init());
        let child = Command::new("cat")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();

        let mut core = Core::new().unwrap();
        let (read, write) = AsyncChildIo::new(child, &core.handle()).unwrap().split();

        let w = WritePoller {
            count: 0,
            inner: write,
        };

        let buf = vec![0u8; 100];

        let r = ReadPoller {
            count: 0,
            data: buf,
            inner: read,
        };

        let rw = r.select(w).for_each(|_| {
            Ok(())
        });

        core.run(rw).unwrap();
        debug!("asio test returned");
    }

    struct UpcaseCodec;

    impl Codec for UpcaseCodec {
        type In = String;
        type Out = String;

        fn decode(&mut self, buf: &mut EasyBuf) -> StdResult<Option<Self::In>, io::Error> {
            use std::str;
            debug!("received a lowercase string");
            Ok(Some(str::from_utf8(buf.as_slice()).unwrap().to_string().to_uppercase()))
        }

        fn encode(&mut self, msg: Self::Out, buf: &mut Vec<u8>) -> StdResult<(), io::Error> {
            debug!("writing a lowercase string");
            buf.write(msg.as_bytes()).map(|_| ())
        }
    }

    #[test]
    fn async_child_io_can_be_framed() {
        drop(env_logger::init());
        let child = Command::new("cat")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();

        let mut core = Core::new().unwrap();
        let (sink, stream) = AsyncChildIo::new(child, &core.handle()).unwrap().framed(UpcaseCodec).split();

        let lowercase: Vec<Result<String>> = vec!["abc", "def", "ghi", "jkl"].into_iter().map(|s| Ok(s.to_string().to_uppercase())).collect();
        let input_stream = iter(lowercase);

        let mut result_vec = RefCell::new(Vec::new());

        let output_handler = stream.for_each(|s| {
            debug!("framed - read {:?}", s);
            Ok(result_vec.borrow_mut().push(s))
        });

        let fut = input_stream.forward(sink).then(|_| output_handler);

        core.run(fut).unwrap();

        assert_eq!(result_vec.borrow_mut().as_slice(), ["ABC", "DEF", "GHI", "JKL"]);
    }

}
