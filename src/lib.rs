#![feature(proc_macro)]
#![feature(field_init_shorthand)]
#![feature(conservative_impl_trait)]

#[macro_use] extern crate chomp;
extern crate futures;
extern crate libc;
#[macro_use] extern crate log;
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
use tokio_core::reactor::{Core, PollEvented};
use std::os::unix::io::AsRawFd;
use language_server_io::AsyncChildIo;
use mio::unix::EventedFd;
use services::{RpcClient};
use worker::Worker;
use uuid::Uuid;
use messages::{Notification, RequestMessage};
use futures::sync::mpsc;
use tokio_service::Service;
use futures::stream::Stream;
use serde_json as json;
use serde_json::builder::{ObjectBuilder};
use std::env;
use services::RequestHandle;
use std::rc::Rc;
use std::cell::RefCell;

pub struct LanguageServer {
    client: RpcClient,
    pub core: Core,
    pub notifications: Box<Stream<Item=Notification, Error=()>>,
}

impl LanguageServer {
    pub fn new<L: Language>(lang: L) -> CustomResult<Self> {
        let core = Core::new()?;
        let handle = core.handle();

        let args = lang.get_command();
        let mut child = Command::new(&args[0])
            .args(&args[1..])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;

        let stdin = child.stdin.unwrap();
        let stdin_fd = stdin.as_raw_fd();
        let stdout = child.stdout.unwrap();
        let stdout_fd = stdout.as_raw_fd();
        PollEvented::new(EventedFd(&stdin_fd), &handle)?;
        PollEvented::new(EventedFd(&stdout_fd), &handle)?;
        let lsio = AsyncChildIo::new(stdin, stdout).into_lsio();

        let (responses_sender, responses_receiver) = mpsc::unbounded();
        let (notifications_queue, notifications_receiver) = mpsc::unbounded();

        let client = RpcClient::new(lsio.clone(), responses_receiver);
        let worker = Worker::new(lsio, notifications_queue, responses_sender);

        core.handle().spawn(worker);

        debug!("server start up");

        Ok(LanguageServer { client, core, notifications: Box::new(notifications_receiver) })
    }

    pub fn initialize(&self) -> RequestHandle {
        unsafe {
            let pid = libc::getpid();
            let cwd = env::current_dir().unwrap();
            self.client.call(RequestMessage {
                id: Uuid::new_v4(),
                method: "initialize".to_string(),
                params: json::to_value(
                    ObjectBuilder::new()
                        .insert("processId", pid)
                        .insert("rootPath", cwd)
                        .insert_object("clientCapabilities", |builder| builder)
                        .build())
            })
        }
    }
}
