use futures::{Async, Future, Poll};
use futures::stream::{Stream};
use messages::{IncomingMessage, ResponseMessage, Notification};
use futures::sync::mpsc;
use serde_json::Value;
use std::io;
use dispatcher::handle_raw_message;
use futures::sink::Sink;

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
            notifications, responses_sink, server_output,
        }
    }
}

impl<T: Stream<Item=Value, Error=io::Error>> Future for Worker<T> {
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        if let Async::NotReady = self.responses_sink.poll_complete().map_err(|_| ())? {
            return Ok(Async::NotReady)
        }

        if let Async::NotReady = self.notifications.poll_complete().map_err(|_| ())? {
            return Ok(Async::NotReady)
        }

        if let Ok(Async::Ready(Some(message))) = self.server_output.poll() {
            match handle_raw_message(message).map_err(|_| ())? {
                IncomingMessage::Response(response) => {
                    self.responses_sink.start_send(response).unwrap();
                },
                IncomingMessage::Notification(notification) => {
                    self.notifications.start_send(notification).unwrap();
                },
                IncomingMessage::MultipleMessages(_) => panic!(),
            }
        }

        Ok(Async::NotReady)
    }
}

#[cfg(test)]
mod test {
    use super::Worker;
    use futures::sync::mpsc;
    use futures::stream::{Stream, iter};
    use tokio_core::reactor::{Core};
    use messages::{Notification, ResponseMessage};
    use uuid::Uuid;
    use serde_json::to_value;

    #[test]
    fn worker_can_dispatch_requests() {
        let (notifications_sender, _) = mpsc::unbounded();
        let (responses_sender, responses_receiver) = mpsc::unbounded();
        let mut core = Core::new().unwrap();

        let response = ResponseMessage {
            id: Uuid::new_v4(),
            result: "never gonna give you up".to_string(),
            error: None
        };

        let stream = iter(vec!(Ok(to_value(response.clone()))));

        let worker = Worker::new(notifications_sender, responses_sender, stream);
        core.handle().spawn(worker);

        if let Ok(dispatched_response) = core.run(responses_receiver.into_future()) {
            assert_eq!(dispatched_response.0.unwrap(), response);
        } else {
            panic!();
        }
    }

    #[test]
    fn worker_can_dispatch_notifications() {
        let (notifications_sender, notifications_receiver) = mpsc::unbounded();
        let (responses_sender, _) = mpsc::unbounded();
        let mut core = Core::new().unwrap();

        let notification = Notification {
            method: "rick".to_string(),
            params: "astley".to_string(),
        };

        let stream = iter(vec!(Ok(to_value(notification.clone()))));

        let worker = Worker::new(notifications_sender, responses_sender, stream);
        core.handle().spawn(worker);

        if let Ok(dispatched_notification) = core.run(notifications_receiver.into_future()) {
            assert_eq!(dispatched_notification.0.unwrap(), notification);
        } else {
            panic!();
        }
    }
}
