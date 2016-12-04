use serde_json::{Map, Value, from_value};
use messages;
use messages::IncomingMessage;
use std::iter::IntoIterator;
use error::Error;

fn handle_object(json_object: Map<String, Value>) -> Result<IncomingMessage, Error> {
    if json_object.get("id").is_some() {
        let deserialized_response =
            from_value::<messages::ResponseMessage>(Value::Object(json_object.clone()))?;
        debug!("is a response");
        Ok(IncomingMessage::Response(deserialized_response))
    } else {
        debug!("is a notification");
        Ok(IncomingMessage::Notification(from_value::<messages::Notification>(Value::Object(json_object.clone()))?))
    }
}


pub fn handle_raw_message(raw_message: Value) -> Result<IncomingMessage, Error> {
    debug!("handling raw message {:?}", raw_message);
    match raw_message {
        Value::Object(message) => handle_object(message),
        Value::Array(messages) => messages.into_iter().map(handle_raw_message).collect(),
        _ => Err(Error::OOL),
    }
}

#[cfg(test)]
mod tests {

    use messages;
    use super::handle_raw_message;
    use messages::IncomingMessage;
    use serde_json::builder;

    #[test]
    fn handle_raw_message_works_with_arrays_of_messages() {
        let first_message = builder::ObjectBuilder::new()
            .insert("jsonrpc", "2.0")
            .insert("id", "48616c6c-6f20-7275-7374-206568206568")
            .insert("result", "frobnicate")
            .build();
        let second_message = builder::ObjectBuilder::new()
            .insert("jsonrpc", "2.0")
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
                assert_eq!(notification,
                           &messages::Notification {
                               method: "combobulate".to_string(),
                               params: "baz".to_string(),
                           });
            } else {
                panic!("Was not a Notification")
            }
        } else {
            panic!("Was not a MultipleMessages")
        }
    }

}
