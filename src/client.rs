use futures::{Async, AsyncSink, Future, Poll, Sink};
use futures::stream::{Peekable, SplitSink, Stream};
use tokio_service::Service;
use messages::{RequestMessage, ResponseMessage};
use error::Error;
use uuid::Uuid;
use std::cell::RefCell;
use std::rc::Rc;
use language_server_io::AsyncChildIo;
use tokio_core::io::Framed;
use codec::RpcCodec;
use evented_receiver::EventedReceiver;

type Responses = Rc<RefCell<Peekable<EventedReceiver<ResponseMessage>>>>;
type ServerInput = Rc<RefCell<SplitSink<Framed<AsyncChildIo, RpcCodec>>>>;

pub struct RequestHandle {
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

        let next_id = {
            match self.responses.borrow_mut().peek()? {
                Async::Ready(Some(msg)) => Some(msg.id),
                Async::Ready(None) => return Err(Error::OOL),
                Async::NotReady => None,
            }
        };

        debug!("next id is {:?}", next_id);

        match next_id {
            Some(id) => {
                if id == self.id {
                    match self.responses.borrow_mut().poll()? {
                        Async::Ready(Some(message)) => Ok(Async::Ready(message)),
                        _ => unreachable!(),
                    }
                } else {
                    Ok(Async::NotReady)
                }
            }
            None => Ok(Async::NotReady),
        }
    }
}

pub struct RpcClient {
    server_input: ServerInput,
    responses: Responses,
}

impl RpcClient {
    pub fn new(server_input: SplitSink<Framed<AsyncChildIo, RpcCodec>>,
               responses: EventedReceiver<ResponseMessage>)
               -> RpcClient {
        RpcClient {
            server_input: Rc::new(RefCell::new(server_input)),
            responses: Rc::new(RefCell::new(responses.peekable())),
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
            id: request.id,
            request: Some(request),
            responses: self.responses.clone(),
            server_input: self.server_input.clone(),
        }
    }
}

#[cfg(test)]
mod test {
    extern crate env_logger;

    use super::RpcClient;
    use uuid::Uuid;
    use messages::{RequestMessage, ResponseMessage};
    use tokio_service::Service;
    use futures::future::*;
    use futures::stream::Stream;
    use tokio_core::reactor::{Core, PollEvented, Timeout};
    use language_server_io::AsyncChildIo;
    use std::process::{Command, Stdio};
    use serde_json as json;
    use tokio_core::io::Io;
    use codec::RpcCodec;
    use mio;
    use evented_receiver::EventedReceiver;
    use std::time::Duration;
    use std::rc::Rc;
    use std::cell::RefCell;

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
        let (_, receiver) = mio::channel::channel();
        let responses = EventedReceiver::new(PollEvented::new(receiver, &core.handle()).unwrap());
        let client = RpcClient::new(sink, responses);
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
        drop(env_logger::init());

        let mut core = Core::new().unwrap();
        let child = Command::new("cat")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        debug!("started cat");
        let (sender, receiver) = mio::channel::channel();
        let responses = EventedReceiver::new(PollEvented::new(receiver, &core.handle()).unwrap());
        let (sink, _) = AsyncChildIo::new(child, &core.handle())
            .unwrap()
            .framed(RpcCodec)
            .split();
        let client = RpcClient::new(sink, responses);

        let request = RequestMessage::new("test_method".to_string(), json::to_value(""));
        let response = ResponseMessage {
            jsonrpc: "2.0".to_string(),
            id: request.id,
            result: Some(json::to_value("never gonna give you up")),
            error: None,
        };
        let expected_response = response.clone();
        let future = client.call(request);

        let request_2 = RequestMessage::new("rickroll".to_string(), json::to_value(""));
        let response_2 = ResponseMessage {
            jsonrpc: "2.0".to_string(),
            id: request_2.id,
            result: Some(json::to_value("never gonna let you down")),
            error: None,
        };
        let expected_response_2 = response_2.clone();
        let future_2 = client.call(request_2);

        let shared_sender = Rc::new(RefCell::new(sender));
        let shared_sender_clone = shared_sender.clone();
        let handle = core.handle();

        let send_responses = Timeout::new(Duration::from_millis(20), &core.handle())
            .unwrap()
            .then::<_, Result<(), ()>>(move |_| {
                debug!("sending response_2 now");
                shared_sender.borrow_mut().send(response_2).unwrap();
                Ok(())
            })
            .then(move |_| Timeout::new(Duration::from_millis(20), &handle).unwrap())
            .then::<_, Result<(), ()>>(move |_| {
                debug!("sending response now");
                shared_sender_clone.borrow_mut().send(response).unwrap();
                Ok(())
            });

        core.handle().spawn(send_responses);

        // Needs to be in this order because we are running them synchronously here
        assert_eq!(core.run(future_2).unwrap(), expected_response_2);
        assert_eq!(core.run(future).unwrap(), expected_response);
    }
}
