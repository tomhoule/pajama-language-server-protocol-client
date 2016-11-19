#![feature(proc_macro)]
#![feature(field_init_shorthand)]

#[macro_use] extern crate chomp;
extern crate futures;
extern crate mio;
extern crate serde;
#[macro_use] extern crate serde_derive;
extern crate serde_json;
extern crate tokio_core;
extern crate tokio_service;
extern crate uuid;

mod dispatcher;
mod error;
mod message_parser;
mod messages;

use futures::Future;
use mio::channel;
use std::process::{Child, Command};
use std::io;
use std::io::{Read, Write};
use tokio_core::reactor::Core;
use tokio_service::Service;
use uuid::Uuid;

use messages::Notification;

struct RpcClient;

struct RequestMessage {
    id: Uuid,
    method: String,
    params: String,
}

struct ResponseMessage<T> {
    id: String,
    result: String,
    error: T,
}


struct ResponseError {
    id: String,
    error: String,
}

impl Service for RpcClient {
    type Request = RequestMessage;
    type Response = ResponseMessage<()>;
    type Error = ResponseMessage<String>;
    type Future = Box<Future<Item=Self::Response, Error=Self::Error>>;

    fn call(&self, request: Self::Request) -> Self::Future {
        // send to the language server stdin, listen for responses
        unimplemented!();
    }
}

impl Service for NotificationServer {
    type Request = Notification;
    type Response = ();
    type Error = ();
    type Future = Box<Future<Item=Self::Response, Error=()>>;

    fn call(&self, request: Self::Request) -> Self::Future {
        // here, just handle the notification
        match request {
            _ => unimplemented!()
        }
    }
}

struct NotificationServer;

trait Language {
    fn start_language_server(&self) -> Result<Child, io::Error>;
}

struct Typescript;

impl Language for Typescript {
    fn start_language_server(&self) -> Result<Child, io::Error> {
        Command::new("sh")
            .arg("-c")
            .arg("tsserver").spawn()
    }
}

struct LanguageServer {
    client: RpcClient,
    notification_server: NotificationServer,
    // child_receiver: channel::Receiver<String>,
    reactor: Core,
}

impl LanguageServer {
    pub fn new<L: Language>(lang: L) -> Result<Self, io::Error> {
        let child = lang.start_language_server()?;
        let reactor = Core::new()?;
        let client = RpcClient {};
        let notification_server = NotificationServer {};
        Ok(LanguageServer { reactor, client, notification_server })
    }
}

#[cfg(test)]
mod tests {

    use super::{LanguageServer, Language};
    use std::io;
    use std::process::{Child, Command};

    struct Cobol;

    impl Language for Cobol {
        fn start_language_server(&self) -> Result<Child, io::Error> {
            Command::new("/bin/sh")
                .arg("-c")
                .arg("echo")
                .arg("hi")
                .spawn()
        }
    }

    #[test]
    fn language_server_new_does_not_panic() {
        LanguageServer::new(Cobol).unwrap();
    }
}
