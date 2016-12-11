use serde_json as json;
use uuid::Uuid;
use std::iter::{FromIterator, IntoIterator};

// #[derive(Debug)]
// enum ErrorCode {
//     ParseError, // -32700
//     InvalidRequest, // -32600
//     MethodNotFound, // -32601
//     InvalidParams, // -32602
//     InternalError, // -32603
//     serverErrorStart, // -32099
//     serverErrorEnd, // -32000
// }


#[derive(Debug)]
pub enum IncomingMessage {
    Response(ResponseMessage),
    Notification(Notification),
    MultipleMessages(Vec<IncomingMessage>),
}

impl FromIterator<IncomingMessage> for IncomingMessage {
    fn from_iter<T>(iter: T) -> Self
        where T: IntoIterator<Item = IncomingMessage>
    {
        IncomingMessage::MultipleMessages(Vec::from_iter(iter))
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RpcError {
    pub code: i32,
    pub message: String,
    pub data: Option<json::Value>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Message {
    pub jsonrpc: String,
    pub id: Option<Uuid>,
    pub method: String,
    pub params: json::Value,
}

impl Message {
    pub fn new_request(method: String, params: json::Value) -> Self {
        Message {
            jsonrpc: "2.0".to_string(),
            id: Some(Uuid::new_v4()),
            method: method,
            params: params,
        }
    }

    pub fn new_notification(method: String, params: json::Value) -> Self {
        Message {
            jsonrpc: "2.0".to_string(),
            id: None,
            method: method,
            params: params,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ResponseMessage {
    pub jsonrpc: String,
    pub id: Uuid,
    pub result: Option<json::Value>,
    pub error: Option<json::Value>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Notification {
    pub jsonrpc: String,
    pub method: String,
    pub params: json::Value,
}

impl Notification {
    pub fn new(method: String, params: json::Value) -> Self {
        Notification {
            jsonrpc: "2.0".to_string(),
            method: method,
            params: params,
        }
    }
}
