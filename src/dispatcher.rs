use serde_json::{Map, Value, from_value, to_value};
use messages;
use messages::RpcError;
use std::iter::{FromIterator, IntoIterator};
use std::result::Result;
use error::Error;
use serde::Deserialize;

#[derive(Debug)]
pub enum IncomingMessage {
    SuccessResponse(messages::ResponseMessage<()>),
    ErrorResponse(messages::ResponseMessage<messages::RpcError>),
    Notification(messages::Notification),
    MultipleMessages(Vec<IncomingMessage>),
}

impl FromIterator<IncomingMessage> for IncomingMessage {
    fn from_iter<T>(iter: T) -> Self
        where T: IntoIterator<Item=IncomingMessage>
        {
            IncomingMessage::MultipleMessages(Vec::from_iter(iter))
        }
}

fn handle_object(json_object: Map<String, Value>) -> Result<IncomingMessage, Error> {
    if let Some(_) = json_object.get("id").clone() {
        match json_object.get("error").clone() {
            Some(stg) => {
                Ok(IncomingMessage::ErrorResponse(from_value::<messages::ResponseMessage<RpcError>>(Value::Object(json_object.clone()))?))
            },
            None => {
                Ok(IncomingMessage::SuccessResponse(from_value::<messages::ResponseMessage<()>>(Value::Object(json_object.clone()))?))
            }
        }
    } else {
        Ok(IncomingMessage::Notification(from_value::<messages::Notification>(Value::Object(json_object.clone()))?))
    }
}


pub fn handle_raw_message(rawMessage: Value) -> Result<IncomingMessage, Error> {
    match rawMessage {
        Value::Object(message) => handle_object(message),
        Value::Array(messages) => messages.into_iter().map(|m| handle_raw_message(m)).collect(),
        _ => Err(Error::new())
    }
}

#[cfg(test)]
mod tests {

    use messages;
    use super::{IncomingMessage, handle_raw_message};
    use serde_json::builder;
    use std::result::Result;

    #[test]
    fn handle_raw_message_works_with_arrays_of_messages() {
        let first_message = builder::ObjectBuilder::new()
            .insert("id", "48616c6c-6f20-7275-7374-206568206568")
            .insert("result", "frobnicate")
            .build();
        let second_message = builder::ObjectBuilder::new()
            .insert("method", "combobulate")
            .insert("params", "baz")
            .build();
        let value = builder::ArrayBuilder::new()
            .push(first_message)
            .push(second_message)
            .build();
        let result = handle_raw_message(value).expect("Could not parse message");

        if let IncomingMessage::MultipleMessages(messages) = result {
            if let IncomingMessage::Notification(ref notification) = messages[1] {
                assert_eq!(
                    notification,
                    &messages::Notification { method: "combobulate".to_string(), params: "baz".to_string() });
            } else {
                panic!("Was not a Notification")
            }
        } else {
            panic!("Was not a MultipleMessages")
        }
    }

    #[test]
    fn handle_raw_message_can_discriminate_between_error_and_success_responses() {
        let success_message = builder::ObjectBuilder::new()
            .insert("id", "48616c6c-6f20-7275-7374-206568206568")
            .insert("result", "frobnicate")
            .build();
        let error_message = builder::ObjectBuilder::new()
            .insert("id", "48616c6c-6f20-7275-7374-206568206568")
            .insert("result", "frobnicate")
            .insert_object("error", |builder| {
                builder.insert("code", 22)
                       .insert("message", "foo".to_string())

            })
            .build();

        match handle_raw_message(success_message) {
            Ok(IncomingMessage::SuccessResponse(_)) => (),
            Err(wrong_result) => panic!(wrong_result),
            wrong_result => panic!(wrong_result)
        }

        match handle_raw_message(error_message) {
            Ok(IncomingMessage::ErrorResponse(_)) => (),
            Err(wrong_result) => panic!(wrong_result),
            wrong_result => panic!(wrong_result)
        }
    }
}
