use std::io;
use tokio_core::io::{Codec, EasyBuf};
use serde_json as json;
use message_parser::parse_message;
use messages::RequestMessage;
use std::io::Write;

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

    fn encode(&mut self, msg: Self::Out, buf: &mut Vec<u8>) -> Result<(), io::Error> {
        let payload = json::to_string(&msg).map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;
        buf.write(format!("Content-Length: {}\r\n\r\n", payload.len()).as_bytes())?;
        debug!("Writing: {}", payload);
        buf.write(payload.as_bytes())?;
        Ok(())
    }
}

