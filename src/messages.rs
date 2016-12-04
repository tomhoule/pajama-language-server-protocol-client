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
pub struct RequestMessage {
    pub jsonrpc: String,
    pub id: Uuid,
    pub method: String,
    pub params: json::Value,
}

impl RequestMessage {
    pub fn new(method: String, params: json::Value) -> Self {
        RequestMessage {
            jsonrpc: "2.0".to_string(),
            id: Uuid::new_v4(),
            method: method,
            params: params,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ResponseMessage {
    pub jsonrpc: String,
    pub id: Uuid,
    pub result: json::Value,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Notification {
    pub method: String,
    pub params: String,
}
