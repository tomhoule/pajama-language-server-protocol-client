use futures::{Async, AsyncSink, Future, Poll, Sink};
use futures::stream::{Peekable, Stream};
use futures::sync::mpsc;
use tokio_service::Service;
use messages::{RequestMessage, ResponseMessage};
use error::{Error};
use uuid::Uuid;
use std::cell::RefCell;
use std::rc::Rc;
use language_server_io::LanguageServerIo;

pub struct RequestHandle {
    id: Uuid,
    request: Option<RequestMessage>,
    server_input: LanguageServerIo,
    receiver: Rc<RefCell<Peekable<mpsc::UnboundedReceiver<ResponseMessage>>>>,
    write_status: Async<()>,
}

impl Future for RequestHandle {
    type Item = ResponseMessage;
    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {

        if let Some(request) = self.request.take() {
            let mut server_input = self.server_input.borrow_mut();
            match server_input.start_send(request)? {
                AsyncSink::Ready => {
                    self.write_status = server_input.poll_complete()?;
                },
                AsyncSink::NotReady(request) => {
                    self.request = Some(request);
                    return Ok(Async::NotReady)
                }
            }
        }

        match self.write_status {
            Async::NotReady => {
                self.write_status = self.server_input.borrow_mut().poll_complete()?;
                if let Async::Ready(()) = self.write_status {
                    ()
                } else {
                    return Ok(Async::NotReady)
                }
            },
            Async::Ready(()) => ()
        }

        debug!("request handle - completed write, {:?}", self.write_status);
        let mut receiver = self.receiver.borrow_mut();

        debug!("request handle - waiting for response");
        match receiver.peek() {
            Ok(Async::Ready(Some(response))) => {
                if response.id != self.id {
                    return Ok(Async::NotReady)
                }
            },
            _ => return Ok(Async::NotReady)
        }

        match receiver.poll() {
            Ok(Async::Ready(Some(response))) => {
                Ok(Async::Ready(response))
            },
            _ => Err(Error::OOL)
        }
    }
}

pub struct RpcClient {
    server_input: LanguageServerIo,
    responses: Rc<RefCell<Peekable<mpsc::UnboundedReceiver<ResponseMessage>>>>,
}

impl RpcClient {
    pub fn new(server_input: LanguageServerIo, receiver: mpsc::UnboundedReceiver<ResponseMessage>) -> RpcClient {
        RpcClient {
            server_input: server_input,
            responses: Rc::new(RefCell::new(receiver.peekable())),
        }
    }
}

impl Service for RpcClient {
    type Request = RequestMessage;
    type Response = ResponseMessage;
    type Error = Error;
    type Future = RequestHandle;

    fn call(&self, request: Self::Request) -> Self::Future {
        let request_id = request.id;
        RequestHandle {
            id: request_id,
            request: Some(request),
            server_input: self.server_input.clone(),
            receiver: self.responses.clone(),
            write_status: Async::NotReady,
        }
    }
}

// #[cfg(test)]
// mod test {
//     use super::RpcClient;
//     use uuid::Uuid;
//     use messages::{RequestMessage, ResponseMessage};
//     use tokio_service::Service;
//     use futures::{Future, Sink};
//     use futures::sync::mpsc;
//     use futures::stream::{iter, Stream};
//     use tokio_core::reactor::{Core};
//     use language_server_io::{AsyncChildIo, LanguageServerIo};
//     use std::process::{Command, Stdio};
//     use serde_json as json;

//     #[test]
//     fn rpc_client_can_be_called() {
//         let core = Core::new().unwrap();
//         let (_, receiver) = mpsc::unbounded();
//         let child = Command::new("/bin/sh")
//             .arg("hi")
//             .stdin(Stdio::piped())
//             .stdout(Stdio::piped())
//             .stderr(Stdio::piped())
//             .spawn()
//             .unwrap();
//         let lsio = AsyncChildIo(child).into_lsio();
//         let client = RpcClient::new(sink, receiver);
//         let request = RequestMessage {
//             id: Uuid::new_v4(),
//             method: "test_method".to_string(),
//             params: json::to_value(""),
//         };
//         let future = client.call(request);
//         core.handle()
//             .spawn(future
//                    .map(|_| ())
//                    .map_err(|_| ()));
//     }

//     #[test]
//     fn rpc_client_can_match_responses_to_requests() {
//         let mut core = Core::new().unwrap();
//         let (sender, receiver) = mpsc::unbounded();
//         let child = Command::new("/bin/sh")
//             .stdin(Stdio::piped())
//             .stdout(Stdio::piped())
//             .stderr(Stdio::piped())
//             .spawn()
//             .unwrap();
//         let (sink, _) = make_io_wrapper(child, core.handle()).unwrap().split();
//         let client = RpcClient::new(sink, receiver);

//         let request_id = Uuid::new_v4();
//         let request = RequestMessage {
//             id: request_id,
//             method: "test_method".to_string(),
//             params: json::to_value(""),
//         };
//         let response = ResponseMessage {
//             id: request_id,
//             result: "never gonna give you up".to_string(),
//             error: None
//         };
//         let future = client.call(request);

//         let request_2_id = Uuid::new_v4();
//         let request_2 = RequestMessage {
//             id: request_2_id,
//             method: "rickroll".to_string(),
//             params: json::to_value(""),
//         };
//         let response_2 = ResponseMessage {
//             id: request_2_id,
//             result: "never gonna let you down".to_string(),
//             error: None
//         };
//         let future_2 = client.call(request_2);

//         sender.send_all(iter(vec!(Ok(response_2.clone()), Ok(response.clone())))).wait().unwrap();

//         // Need to be in this order because we are running them synchronously here
//         assert_eq!(core.run(future_2).unwrap(), response_2);
//         assert_eq!(core.run(future).unwrap(), response);
//     }
// }
