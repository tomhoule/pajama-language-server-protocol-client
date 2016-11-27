use futures::{Async, Future, Poll};
use futures::stream::{Stream, SplitStream};
use language_server_io::IoWrapper;
use messages::{ResponseMessage, Notification};
use futures::sync::mpsc;

pub struct Worker {
    notifications: mpsc::UnboundedSender<Notification>,
    responses_sink: mpsc::UnboundedSender<ResponseMessage>,
    server_output: SplitStream<IoWrapper>,
}

impl Worker {
    pub fn new(notifications: mpsc::UnboundedSender<Notification>, responses_sink: mpsc::UnboundedSender<ResponseMessage>, server_output: SplitStream<IoWrapper>) -> Self {
        Worker {
            notifications, responses_sink, server_output
        }
    }
}

impl Future for Worker {
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        if let Ok(Async::Ready(Some(message))) = self.server_output.poll() {
            println!("{:?}", message)
        }
        Ok(Async::NotReady)
    }
}
