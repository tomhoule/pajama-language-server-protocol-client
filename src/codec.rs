use std::io;
use tokio_core::io::{Codec, EasyBuf};
use serde_json as json;
use message_parser::parse_message;
use messages::{IncomingMessage, OutgoingMessage};
use std::io::Write;
use dispatcher::handle_raw_message;
use std::str;

pub struct RpcCodec;

impl Codec for RpcCodec {
    type In = IncomingMessage;
    type Out = OutgoingMessage;

    fn decode(&mut self, buf: &mut EasyBuf) -> Result<Option<Self::In>, io::Error> {
        let json = {
            match parse_message(buf.as_slice()) {
                Ok(inner) => inner,
                Err(_) => return Ok(None),
            }
        };
        debug!("decode - json value: {:?}", json);
        match json {
            Ok(json_value) => {
                let message = handle_raw_message(json_value)
                    .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;
                debug!("decode - returning message {:?}", message);
                let buf_end = buf.len() - 1;
                buf.drain_to(buf_end);
                Ok(Some(message))
            }
            Err(parse_error) => {
                debug!("decode - couldn't parse: {:?}", parse_error);
                Err(io::Error::new(io::ErrorKind::InvalidData, parse_error))
            }
        }
    }

    fn encode(&mut self, msg: Self::Out, buf: &mut Vec<u8>) -> Result<(), io::Error> {
        let payload = match msg {
            OutgoingMessage::Request(ref req) => json::to_string(req),
            OutgoingMessage::Notification(ref notification) => json::to_string(notification),
            OutgoingMessage::MultipleMessages(_) => unimplemented!()
        }.map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;
        buf.write(format!("Content-Length: {}\r\n\r\n", payload.len()).as_bytes())?;
        debug!("encode - writing: {}", payload);
        buf.write(payload.as_bytes())?;
        Ok(())
    }
}
