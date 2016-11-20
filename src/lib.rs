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
mod language;
mod message_parser;
mod messages;
mod language_server_io;

pub use language::Language;

use futures::Future;
use mio::channel;
use std::process::{Command, Child, Stdio};
use std::io;
use tokio_core::reactor::{Core, PollEvented};
use tokio_service::Service;
use uuid::Uuid;
use messages::{RequestMessage, ResponseMessage, RpcError};
use language_server_io::LanguageServerIo;

use messages::Notification;

struct RpcClient;

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


pub struct LanguageServer {
    client: RpcClient,
    core: Core,
    notification_server: NotificationServer,
    poll_evented: PollEvented<LanguageServerIo>,
}

impl LanguageServer {
    pub fn new<L: Language>(lang: L) -> Result<Self, io::Error> {
        let args = lang.get_command();
        let child = Command::new(&args[0])
            .args(&args[1..])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        let core = Core::new()?;
        let poll_evented = PollEvented::new(LanguageServerIo::new(child), &core.handle())?;
        let client = RpcClient {};
        let notification_server = NotificationServer {};
        Ok(LanguageServer { core, client, notification_server, poll_evented })
    }

    pub fn stop(self) -> Result<(), io::Error> {
        self.poll_evented.deregister(&self.core.handle());
        Ok(())
    }
}
