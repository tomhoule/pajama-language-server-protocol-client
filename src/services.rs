use futures::{Async, Future, Poll};
use tokio_service::Service;
use messages::{Notification, RequestMessage, ResponseMessage};
use error::{Error};
use uuid::Uuid;
use std::collections::HashMap;
use std::cell::RefCell;
use std::rc::Rc;
use crossbeam::sync::MsQueue;

pub struct RequestHandle {
    id: Uuid,
    map: Rc<RefCell<HashMap<Uuid, ResponseMessage>>>,
}

impl Future for RequestHandle {
    type Item = ResponseMessage;
    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let mut map = self.map.borrow_mut();
        if let Some(response) = map.remove(&self.id) {
            Ok(Async::Ready(response))
        } else {
            Ok(Async::NotReady)
        }
    }
}

pub struct RpcClient {
    running_requests: Rc<RefCell<HashMap<Uuid, ResponseMessage>>>,
}

impl RpcClient {
    pub fn new() -> RpcClient {
        RpcClient {
            running_requests: Rc::new(RefCell::new(HashMap::<Uuid, ResponseMessage>::new()))
        }
    }

    pub fn handle_response(&self, response: ResponseMessage) {
        let mut map = self.running_requests.borrow_mut();
        map.insert(response.id, response);
    }
}

impl Service for RpcClient {
    type Request = RequestMessage;
    type Response = ResponseMessage;
    type Error = Error;
    type Future = Box<Future<Item=Self::Response, Error=Self::Error>>;

    fn call(&self, request: Self::Request) -> Self::Future {
        // here send to the language server stdin
        Box::new(RequestHandle {
            id: request.id.clone(),
            map: self.running_requests.clone()
        })
    }
}

pub type NotificationServer = MsQueue<Notification>;

#[cfg(test)]
mod test {
    use super::RpcClient;
    use uuid::Uuid;
    use messages::{RequestMessage, ResponseMessage};
    use tokio_service::Service;
    use futures::Future;
    use tokio_core::reactor::{Core, Handle};

    #[test]
    fn rpc_client_can_be_called() {
        let client = RpcClient::new();
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
        let client = RpcClient::new();
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

        client.handle_response(response_2.clone());
        client.handle_response(response.clone());

        assert_eq!(core.run(future).unwrap(), response);
        assert_eq!(core.run(future_2).unwrap(), response_2);
    }
}
