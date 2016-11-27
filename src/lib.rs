#![feature(proc_macro)]
#![feature(field_init_shorthand)]

#[macro_use] extern crate chomp;
extern crate crossbeam;
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
use language_server_io::{make_io_wrapper, IoWrapper};
use services::{NotificationServer, RpcClient};
use worker::Worker;

pub struct LanguageServer {
    client: RpcClient,
    core: Core,
    notification_server: NotificationServer,
    interface: IoWrapper,
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
        let interface = make_io_wrapper(child, core.handle())?;
        let client = RpcClient::new(interface.clone());
        let notification_server = NotificationServer::new();
        Ok(LanguageServer { core, client, notification_server, interface })
    }

    pub fn start(&mut self) -> Result<(), ()> {
        let worker = Worker::new(&self.notification_server, &self.client, self.interface.clone());
        self.core.run(worker)
    }
}
