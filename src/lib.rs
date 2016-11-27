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
use std::rc::Rc;


/// To be added:
/// The interface member will disappear, because it will be split and sent into the worker for the
/// read half and the client for the write half.
/// Notifications should just be a receiver, (but with a queue api?)
/// We need the RpcCient to have a responselistener (?), also a future, that reads from the worker
/// channel and handles the responses. Maybe an impl future for client itself?
pub struct LanguageServer {
    client: Rc<RpcClient>,
    core: Core,
    pub notifications: Rc<NotificationServer>,
    interface: Rc<IoWrapper>,
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
        let interface = Rc::new(make_io_wrapper(child, core.handle())?);
        let client = Rc::new(RpcClient::new(interface.clone()));
        let notifications = Rc::new(NotificationServer::new());
        Ok(LanguageServer { core, client, notifications, interface })
    }

    pub fn start(&mut self) {
        let worker = Worker::new(self.notifications.clone(), self.client.clone(), self.interface.clone());
        self.core.handle().spawn(worker)
    }
}
