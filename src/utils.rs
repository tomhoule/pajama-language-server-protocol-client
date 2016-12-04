use messages::ResponseMessage;
use serde_json::from_value;
use serde::Deserialize;
use error::Error;

pub fn handle_response<R, E>(response: ResponseMessage) -> Result<Result<R, E>, Error>
    where R: Deserialize,
          E: Deserialize
{
    match (response.result, response.error) {
        (Some(result), None) => Ok(Ok(from_value::<R>(result)?)),
        (None, Some(error)) => Ok(Err(from_value::<E>(error)?)),
        _ => Err(Error::OOL),
    }
}
