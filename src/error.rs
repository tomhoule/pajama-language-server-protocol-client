use std::convert::From;
use std::result::Result as StdResult;
use serde_json;

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::Deserialization(err)
    }
}

pub type Result<T> = StdResult<T, Error>;

#[derive(Debug)]
pub enum Error {
    Deserialization(serde_json::Error),
    OOL
}

impl From<()> for Error {
    fn from(_: ()) -> Self {
        Error::OOL
    }
}
