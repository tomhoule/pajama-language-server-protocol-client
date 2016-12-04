use crossbeam::sync::MsQueue;
use futures::{Async, AsyncSink, Future, Poll, Sink};
use futures::stream::SplitSink;
use tokio_service::Service;
use messages::{RequestMessage, ResponseMessage};
use error::Error;
use uuid::Uuid;
use std::cell::RefCell;
use std::rc::Rc;
use language_server_io::AsyncChildIo;
use tokio_core::io::Framed;
use codec::RpcCodec;

type Responses = Rc<MsQueue<ResponseMessage>>;
type ServerInput = Rc<RefCell<SplitSink<Framed<AsyncChildIo, RpcCodec>>>>;

pub struct RequestHandle {
    current_response: Rc<RefCell<Option<ResponseMessage>>>,
    id: Uuid,
    request: Option<RequestMessage>,
    responses: Responses,
    server_input: ServerInput,
}

impl Future for RequestHandle {
    type Item = ResponseMessage;
    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        debug!("polling for a response, {:?}", self.id);
        let mut server_input = self.server_input.borrow_mut();
        if let Some(request) = self.request.take() {
            match server_input.start_send(request)? {
                AsyncSink::Ready => (),
                AsyncSink::NotReady(req) => {
                    self.request = Some(req);
                    return Ok(Async::NotReady);
                }
            }
        }

        match server_input.poll_complete()? {
            Async::Ready(()) => (),
            Async::NotReady => return Ok(Async::NotReady),
        }

        debug!("trying to match a response to request {:?}", self.id);
        let mut current_response = self.current_response.borrow_mut();
        if current_response.is_none() {
            match self.responses.try_pop() {
                Some(next) => {
                    if next.id == self.id {
                        Ok(Async::Ready(next))
                    } else {
                        *current_response = Some(next);
                        Ok(Async::NotReady)
                    }
                }
                None => return Ok(Async::NotReady),
            }
        } else {
            let response = current_response.take().unwrap();
            if response.id == self.id {
                Ok(Async::Ready(response))
            } else {
                Ok(Async::NotReady)
            }
        }
    }
}

pub struct RpcClient {
    server_input: ServerInput,
    responses: Responses,
    current_response: Rc<RefCell<Option<ResponseMessage>>>,
}

impl RpcClient {
    pub fn new(server_input: SplitSink<Framed<AsyncChildIo, RpcCodec>>,
               responses: Responses)
               -> RpcClient {
        RpcClient {
            server_input: Rc::new(RefCell::new(server_input)),
            responses: responses,
            current_response: Rc::new(RefCell::new(None)),
        }
    }
}

impl Service for RpcClient {
    type Request = RequestMessage;
    type Response = ResponseMessage;
    type Error = Error;
    type Future = RequestHandle;

    fn call(&self, request: Self::Request) -> Self::Future {
        RequestHandle {
            current_response: self.current_response.clone(),
            id: request.id,
            request: Some(request),
            responses: self.responses.clone(),
            server_input: self.server_input.clone(),
        }
    }
}

#[cfg(test)]
mod test {
    use super::RpcClient;
    use uuid::Uuid;
    use messages::{RequestMessage, ResponseMessage};
    use tokio_service::Service;
    use futures::Future;
    use futures::stream::Stream;
    use tokio_core::reactor::Core;
    use language_server_io::AsyncChildIo;
    use std::process::{Command, Stdio};
    use serde_json as json;
    use tokio_core::io::Io;
    use crossbeam::sync::MsQueue;
    use codec::RpcCodec;
    use std::rc::Rc;

    #[test]
    fn rpc_client_can_be_called() {
        let core = Core::new().unwrap();
        let child = Command::new("/bin/sh")
            .arg("hi")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let (sink, _) = AsyncChildIo::new(child, &core.handle())
            .unwrap()
            .framed(RpcCodec)
            .split();
        let responses = MsQueue::new();
        let client = RpcClient::new(sink, Rc::new(responses));
        let request = RequestMessage {
            jsonrpc: "2.0".to_string(),
            id: Uuid::new_v4(),
            method: "test_method".to_string(),
            params: json::to_value(""),
        };
        let future = client.call(request);
        core.handle()
            .spawn(future.map(|_| ())
                .map_err(|_| ()));
    }

    #[test]
    fn rpc_client_can_match_responses_to_requests() {
        let mut core = Core::new().unwrap();
        let child = Command::new("cat")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let responses = MsQueue::new();
        let (sink, _) = AsyncChildIo::new(child, &core.handle())
            .unwrap()
            .framed(RpcCodec)
            .split();
        let client = RpcClient::new(sink, Rc::new(responses));

        let request = RequestMessage::new("test_method".to_string(), json::to_value(""));
        let response = ResponseMessage {
            jsonrpc: "2.0".to_string(),
            id: request.id,
            result: json::to_value("never gonna give you up"),
            error: None,
        };
        let future = client.call(request);

        let request_2 = RequestMessage::new("rickroll".to_string(), json::to_value(""));
        let response_2 = ResponseMessage {
            jsonrpc: "2.0".to_string(),
            id: request_2.id,
            result: json::to_value("never gonna let you down"),
            error: None,
        };
        let future_2 = client.call(request_2);

        client.responses.push(response_2.clone());
        client.responses.push(response.clone());

        // Need to be in this order because we are running them synchronously here
        assert_eq!(core.run(future_2).unwrap(), response_2);
        assert_eq!(core.run(future).unwrap(), response);
    }
}
