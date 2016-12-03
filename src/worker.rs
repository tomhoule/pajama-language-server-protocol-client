use futures::{Async, Future, Poll};
use futures::stream::{Stream};
use messages::{IncomingMessage, ResponseMessage, Notification};
use futures::sync::mpsc;
use serde_json::Value;
use std::io;
use dispatcher::handle_raw_message;
use futures::sink::Sink;
use language_server_io::LanguageServerIo;
use tokio_core::io::Io;
use std::io::Read;
use error::Error;

pub struct Worker<T>
where T: Stream<Item=Value, Error=io::Error> {
    inbound: T,
    notifications_sink: mpsc::UnboundedSender<Notification>,
    responses_sink: mpsc::UnboundedSender<ResponseMessage>,
}

impl<T: Stream<Item=Value, Error=io::Error>> Worker<T> {
    pub fn new(
        inbound: T,
        notifications_sink: mpsc::UnboundedSender<Notification>,
        responses_sink: mpsc::UnboundedSender<ResponseMessage>) -> Self
    {
        Worker {
            notifications_sink, responses_sink, inbound,
        }
    }
}

impl<T: Stream<Item=Value, Error=io::Error>> Stream for Worker<T> {
    type Item = ();
    type Error = Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        debug!("worker - polling for writes on sinks");
        // self.responses_sink.poll_complete();
        // self.notifications_sink.poll_complete();

        debug!("worker - polling for reads on process stdout");
        if let Async::Ready(Some(message)) = self.inbound.poll()? {
            debug!("got something!");
            match handle_raw_message(message).map_err(|_| ())? {
                IncomingMessage::Response(response) => {
                    debug!("worker - incoming response");
                    self.responses_sink.start_send(response);
                },
                IncomingMessage::Notification(notification) => {
                    debug!("worker - incoming notification");
                    self.notifications_sink.start_send(notification);
                },
                IncomingMessage::MultipleMessages(_) => {
                    debug!("worker - incoming multiple messages");
                }
            }
            Ok(Async::Ready(Some(())))
        } else {
            debug!("worker - polling for reads on process stdout");
            Ok(Async::NotReady)
        }
    }
}

// #[cfg(test)]
// mod test {
//     use super::Worker;
//     use futures::*;
//     use futures::sync::mpsc;
//     use futures::stream::{Stream, iter};
//     use tokio_core::reactor::{Core};
//     use messages::{Notification, RequestMessage};
//     use uuid::Uuid;
//     use serde_json::to_value;
//     use std::process::*;
//     use language_server_io::AsyncChildIo;
//     use futures::sink::Sink;
//     use tokio_core::io::Io;
//     use codec::RpcCodec;

//     #[test]
//     fn worker_can_dispatch_requests() {
//         let (responses_sender, _) = mpsc::unbounded();
//         let (notifications_sender, _) = mpsc::unbounded();
//         let child = Command::new("cat")
//             .stdin(Stdio::piped())
//             .stdout(Stdio::piped())
//             .spawn()
//             .unwrap();

//         let mut core = Core::new().unwrap();

//         // let response = ResponseMessage {
//         //     id: Uuid::new_v4(),
//         //     result: "never gonna give you up".to_string(),
//         //     error: None
//         // };

//         let request = RequestMessage {
//             id: Uuid::new_v4(),
//             method: "rick".to_string(),
//             params: to_value("astley"),
//         };

//         let (sink, stream) = AsyncChildIo::new(child, &core.handle()).unwrap().framed(RpcCodec).split();

//         let worker = Worker::new(stream, notifications_sender, responses_sender).for_each(|_| {
//             debug!("handling a new message");
//             Ok(())
//         });

//         // let fut = sink.send(request).then(|_| worker);

//         core.run(worker).unwrap();
//     }

//     // #[test]
//     // fn worker_can_dispatch_notifications() {
//     //     let (notifications_sender, notifications_receiver) = mpsc::unbounded();
//     //     let (responses_sender, _) = mpsc::unbounded();
//     //     let mut core = Core::new().unwrap();

//     //     let notification = Notification {
//     //         method: "rick".to_string(),
//     //         params: "astley".to_string(),
//     //     };

//     //     let stream = iter(vec!(Ok(to_value(notification.clone()))));

//     //     let worker = Worker::new(notifications_sender, responses_sender, stream);
//     //     core.handle().spawn(worker);

//     //     if let Ok(dispatched_notification) = core.run(notifications_receiver.into_future()) {
//     //         assert_eq!(dispatched_notification.0.unwrap(), notification);
//     //     } else {
//     //         panic!();
//     //     }
//     // }
// }
