use futures::{Async, Future, Poll};
use futures::stream::{Stream};
use crossbeam::sync::MsQueue;
use services::RpcClient;
use language_server_io::IoWrapper;
use messages::Notification;
use std::ops::Deref;
use std::rc::Rc;

/// When Framed.split will be stabilized in tokio_core, we can make this static by using channels
/// for notifications and responses, and taking ownership of the FramedRead here
pub struct Worker {
    notifications: Rc<MsQueue<Notification>>,
    client: Rc<RpcClient>,
    io: Rc<IoWrapper>,
}

impl Worker {
    pub fn new(notifications: Rc<MsQueue<Notification>>, client: Rc<RpcClient>, io: Rc<IoWrapper>) -> Self {
        Worker {
            notifications, client, io
        }
    }
}

impl Future for Worker {
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        if let Some(io) = Rc::get_mut(&mut self.io) {
            if let Ok(Async::Ready(Some(message))) = io.poll() {
                println!("{:?}", message)
            }
        }
        Ok(Async::NotReady)
    }
}
