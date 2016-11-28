use futures::{Async, Future, Poll};
use futures::stream::{Stream, SplitStream};
use language_server_io::IoWrapper;
use messages::{IncomingMessage, ResponseMessage, Notification};
use error::Error;
use futures::sync::mpsc;
use serde_json::Value;
use std::io;
use dispatcher::handle_raw_message;

pub struct Worker<T: Stream<Item=Value, Error=io::Error>> {
    notifications: mpsc::UnboundedSender<Notification>,
    responses_sink: mpsc::UnboundedSender<ResponseMessage>,
    server_output: T,
}

impl<T: Stream<Item=Value, Error=io::Error>> Worker<T> {
    pub fn new(
        notifications: mpsc::UnboundedSender<Notification>,
        responses_sink: mpsc::UnboundedSender<ResponseMessage>,
        server_output: T) -> Self
    {
        Worker {
            notifications, responses_sink, server_output
        }
    }
}

impl<T: Stream<Item=Value, Error=io::Error>> Future for Worker<T> {
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        if let Ok(Async::Ready(Some(message))) = self.server_output.poll() {
            match handle_raw_message(message)? {
                IncomingMessage::Response(response) => (),
                IncomingMessage::Notification(notification) => (),
                IncomingMessage::MultipleValues(values) => (),
            }
        }

        Ok(Async::NotReady)
    }
}

#[cfg(test)]
mod test {
    use super::Worker;
    use language_server_io::make_io_wrapper;
    use futures::sync::mpsc;
    use futures::stream::{Stream, iter};
    use futures::{Async, Future};
    use tokio_core::reactor::{Core, Interval};
    use std::process::{Command, Stdio};
    use messages::{IncomingMessage, Notification, ResponseMessage};
    use futures::sink::Sink;
    use uuid::Uuid;
    use serde_json::to_value;
    use std::time::Duration;

    #[test]
    fn worker_can_read_and_discriminate_responses_and_notifications() {
        let (notifications_sender, mut notifications_receiver) = mpsc::unbounded();
        let (responses_sender, mut responses_receiver) = mpsc::unbounded();
        let mut core = Core::new().unwrap();
        Interval::new(Duration::new(1, 0), &core.handle());

        let notification = Notification {
            method: "rick".to_string(),
            params: "astley".to_string(),
        };

        let response = ResponseMessage {
            id: Uuid::new_v4(),
            result: "never gonna give you up".to_string(),
            error: None
        };

        let stream = iter(vec!(Ok(to_value(notification.clone())), Ok(to_value(response.clone()))));

        let mut worker = Worker::new(notifications_sender, responses_sender, stream);
        core.handle().spawn(worker);

        if let Ok(dispatched_response) = core.run(responses_receiver.into_future()) {
            assert_eq!(dispatched_response.0.unwrap(), response);
        } else {
            panic!();
        }
        // assert_eq!(responses_receiver.poll(), Ok(Async::Ready(None)));
        // assert_eq!(notifications_receiver.poll(), Ok(Async::Ready(Some(notification))));
        // assert_eq!(notifications_receiver.poll(), Ok(Async::Ready(None)));
    }
}
