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

mod codec;
mod dispatcher;
mod error;
mod language;
mod language_server_io;
mod message_parser;
mod messages;
mod services;

pub use language::Language;

use codec::RpcCodec;
use std::process::{Command, Stdio};
use std::io;
use tokio_core::reactor::{Core, PollEvented};
use tokio_core::io::{Io, Framed};
use language_server_io::LanguageServerIo;
use services::{NotificationServer, RpcClient};

pub struct LanguageServer {
    client: RpcClient,
    core: Core,
    notification_server: NotificationServer,
    interface: Framed<PollEvented<LanguageServerIo>, RpcCodec>,
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
        let interface = PollEvented::new(LanguageServerIo::new(child), &core.handle())?.framed(RpcCodec);
        let client = RpcClient::new();
        let notification_server = NotificationServer {};
        Ok(LanguageServer { core, client, notification_server, interface })
    }

    pub fn stop(self) -> Result<(), io::Error> {
        self.interface.into_inner().deregister(&self.core.handle())
    }
}
