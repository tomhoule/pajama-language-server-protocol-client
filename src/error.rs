use std::convert::From;
use std::result::Result as StdResult;
use serde_json;
use std::io;

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::Io(err)
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::Deserialization(err)
    }
}

pub type Result<T> = StdResult<T, Error>;

#[derive(Debug)]
pub enum Error {
    Deserialization(serde_json::Error),
    Io(io::Error),
    OOL
}

impl From<()> for Error {
    fn from(_: ()) -> Self {
        Error::OOL
    }
}
