use futures::{Async, Future, Poll};
use futures::stream::{Stream};
use crossbeam::sync::MsQueue;
use services::RpcClient;
use language_server_io::IoWrapper;
use messages::Notification;
use std::ops::Deref;
use std::rc::Rc;

pub struct Worker {
    notifications: &'static MsQueue<Notification>,
    client: &'static RpcClient,
    io: IoWrapper,
}

impl Worker {
    pub fn new(notifications: &'static MsQueue<Notification>, client: &'static RpcClient, io: IoWrapper) -> Self {
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
