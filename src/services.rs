use futures::{Async, Future, Poll, Sink};
use futures::stream::{Peekable, Stream, SplitSink};
use futures::sync::mpsc;
use tokio_service::Service;
use messages::{Notification, RequestMessage, ResponseMessage};
use error::{Error};
use uuid::Uuid;
use std::cell::RefCell;
use std::rc::Rc;
use language_server_io::IoWrapper;

pub struct RequestHandle {
    id: Uuid,
    receiver: Rc<RefCell<Peekable<mpsc::UnboundedReceiver<ResponseMessage>>>>,
}

impl Future for RequestHandle {
    type Item = ResponseMessage;
    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let mut receiver = self.receiver.borrow_mut();
        {
            match receiver.peek() {
                Ok(Async::Ready(Some(response))) => {
                    if response.id != self.id {
                        return Ok(Async::NotReady)
                    }
                },
                response => {
                    return Ok(Async::NotReady)
                }
            }
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
    server_input: SplitSink<IoWrapper>,
    responses: Rc<RefCell<Peekable<mpsc::UnboundedReceiver<ResponseMessage>>>>,
}

impl RpcClient {
    pub fn new(server_input: SplitSink<IoWrapper>, receiver: mpsc::UnboundedReceiver<ResponseMessage>) -> RpcClient {
        RpcClient {
            server_input,
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
        // self.server_input.start_send(request).unwrap();
        // self.server_input.poll_complete().unwrap();
        RequestHandle {
            id: request.id.clone(),
            receiver: self.responses.clone(),
        }
    }
}

#[cfg(test)]
mod test {
    use super::RpcClient;
    use uuid::Uuid;
    use messages::{RequestMessage, ResponseMessage};
    use tokio_service::Service;
    use futures::{Future, Sink};
    use futures::sync::mpsc;
    use futures::stream::{iter, Stream};
    use tokio_core::reactor::Core;
    use language_server_io::make_io_wrapper;
    use std::process::{Command, Stdio};

    #[test]
    fn rpc_client_can_be_called() {
        let core = Core::new().unwrap();
        let (_, receiver) = mpsc::unbounded();
        let child = Command::new("/bin/echo")
            .arg("testing")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let (sink, _) = make_io_wrapper(child, core.handle()).unwrap().split();
        let client = RpcClient::new(sink, receiver);
        let request = RequestMessage {
            id: Uuid::new_v4(),
            method: "test_method".to_string(),
            params: "".to_string(),
        };
        let mut future = (&client).call(request);
        assert!(future.poll().unwrap().is_not_ready())
    }

    #[test]
    fn rpc_client_can_match_responses_to_requests() {
        let mut core = Core::new().unwrap();
        let (sender, receiver) = mpsc::unbounded();
        let child = Command::new("/usr/bin/tee")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let (sink, _) = make_io_wrapper(child, core.handle()).unwrap().split();
        let client = RpcClient::new(sink, receiver);
        let request_id = Uuid::new_v4();
        let request = RequestMessage {
            id: request_id,
            method: "test_method".to_string(),
            params: "".to_string(),
        };

        let response = ResponseMessage {
            id: request_id,
            result: "never gonna give you up".to_string(),
            error: None
        };

        let future = client.call(request);

        let request_2_id = Uuid::new_v4();

        let request_2 = RequestMessage {
            id: request_2_id,
            method: "rickroll".to_string(),
            params: "".to_string(),
        };

        let response_2 = ResponseMessage {
            id: request_2_id,
            result: "never gonna let you down".to_string(),
            error: None
        };

        let future_2 = client.call(request_2);

        sender.send_all(iter(vec!(Ok(response_2.clone()), Ok(response.clone())))).wait().unwrap();

        // Need to be in this order because we are running them synchronously here
        assert_eq!(core.run(future_2).unwrap(), response_2);
        assert_eq!(core.run(future).unwrap(), response);
    }
}
