use mio::channel::*;
use error::Error;
use tokio_core::reactor::PollEvented;
use futures::stream::Stream;
use futures::Async;
use std::sync::mpsc::TryRecvError;

pub struct EventedReceiver<T> {
    inner: PollEvented<Receiver<T>>,
    read_inner: bool,
}

impl<T> EventedReceiver<T> {
    pub fn new(inner: PollEvented<Receiver<T>>) -> Self {
        EventedReceiver {
            inner: inner,
            read_inner: false,
        }
    }
}

impl<T> Stream for EventedReceiver<T> {
    type Item = T;
    type Error = Error;

    fn poll(&mut self) -> Result<Async<Option<Self::Item>>, Self::Error> {
        if self.read_inner {
            self.inner.need_read();
            self.read_inner = false;
        }

        if let Async::Ready(()) = self.inner.poll_read() {
            match self.inner.get_ref().try_recv() {
                Ok(message) => {
                    self.read_inner = true;
                    Ok(Async::Ready(Some(message)))
                },
                Err(TryRecvError::Empty) => {
                    self.read_inner = true;
                    Ok(Async::NotReady)
                },
                Err(TryRecvError::Disconnected) => Err(Error::OOL)
            }
        } else {
            Ok(Async::NotReady)

        }
    }

}
