#![feature(proc_macro)]
#![feature(field_init_shorthand)]
#![feature(conservative_impl_trait)]

#[macro_use] extern crate chomp;
extern crate futures;
extern crate mio;
extern crate serde;
#[macro_use] extern crate serde_derive;
extern crate serde_json;
extern crate tokio_core;
extern crate tokio_service;
extern crate uuid;

mod codec;
mod dispatcher;
mod error;
mod language;
mod language_server_io;
mod message_parser;
mod messages;
mod services;
mod worker;

pub use language::Language;

use std::process::{Command, Stdio};
use error::Result as CustomResult;
use tokio_core::reactor::Core;
use language_server_io::{make_io_wrapper};
use services::{RpcClient};
use worker::Worker;
use uuid::Uuid;
use messages::{Notification, RequestMessage, ResponseMessage};
use futures::sync::mpsc;
use tokio_service::Service;
use futures::stream::Stream;
use error::Error;
use futures::Future;


pub struct LanguageServer {
    client: RpcClient,
    pub notifications: Box<Stream<Item=Notification, Error=()>>,
}

impl LanguageServer {
    pub fn new<L: Language>(lang: L) -> CustomResult<Self> {
        let args = lang.get_command();
        let child = Command::new(&args[0])
            .args(&args[1..])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        let core = Core::new()?;
        let (responses_sender, responses_receiver) = mpsc::unbounded();
        let (notifications_sender, notifications_receiver) = mpsc::unbounded();
        let (sink, stream) = make_io_wrapper(child, core.handle())?.split();
        let client = RpcClient::new(sink, responses_receiver);

        let worker = Worker::new(notifications_sender, responses_sender, stream);
        core.handle().spawn(worker);

        Ok(LanguageServer { client, notifications: Box::new(notifications_receiver) })
    }

    pub fn initialize(&self) -> Box<Future<Item=ResponseMessage, Error=Error>> {
        self.client.call(RequestMessage {
            id: Uuid::new_v4(),
            method: "initialize".to_string(),
            params: String::new(),
        })
    }
}
