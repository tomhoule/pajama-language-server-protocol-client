use std::io;
use tokio_core::io::{Codec, EasyBuf};
use serde_json as json;
use message_parser::parse_message;
use messages::RequestMessage;

pub struct RpcCodec;

impl Codec for RpcCodec {
    type In = json::Value;
    type Out = RequestMessage;

    fn decode(&mut self, buf: &mut EasyBuf) -> Result<Option<Self::In>, io::Error> {
        let json = parse_message(buf.as_slice());
        match json {
            Ok(inner) => {
                match inner {
                    Ok(json_value) => Ok(Some(json_value)),
                    Err(parse_error) => Err(io::Error::new(io::ErrorKind::InvalidData, parse_error)),
                }
            }
            Err(_) => Ok(None)
        }
    }

    fn encode(&mut self, msg: Self::Out, buf: &mut Vec<u8>) {
        json::to_writer(buf, &msg);
    }
}

