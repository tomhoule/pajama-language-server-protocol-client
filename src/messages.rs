use serde_json as json;
use uuid::Uuid;
use error::{Result, Error};


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

#[derive(Debug, Deserialize)]
pub struct RpcError {
    pub code: i32,
    pub message: String,
    pub data: Option<json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct RequestMessage {
    pub id: Uuid,
    pub method: String,
    pub params: String,
}

#[derive(Debug, Deserialize)]
pub struct ResponseMessage<T> {
    pub id: String,
    pub result: String,
    pub error: Option<T>,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct Notification {
    pub method: String,
    pub params: String,
}
