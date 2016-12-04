#![feature(proc_macro)]
#![feature(field_init_shorthand)]
#![feature(conservative_impl_trait)]

#[macro_use]
extern crate chomp;
extern crate crossbeam;
extern crate futures;
extern crate libc;
#[macro_use]
extern crate log;
extern crate mio;
extern crate serde;
#[macro_use]
extern crate serde_derive;
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

use std::process::{Command, Stdio};
use error::Result as CustomResult;
use tokio_core::reactor::Core;
use language_server_io::AsyncChildIo;
use services::RpcClient;
use uuid::Uuid;
use messages::{Notification, RequestMessage, IncomingMessage};
use tokio_service::Service;
use futures::stream::Stream;
use serde_json as json;
use serde_json::builder::ObjectBuilder;
use std::env;
use services::RequestHandle;
use codec::RpcCodec;
use tokio_core::io::Io;
use std::rc::Rc;
use crossbeam::sync::MsQueue;
use futures::Future;

pub struct LanguageServer {
    client: RpcClient,
    pub core: Core,
    pub notifications: Rc<MsQueue<Notification>>,
}

impl LanguageServer {
    pub fn new<L: Language>(lang: L) -> CustomResult<Self> {
        let core = Core::new()?;
        let handle = core.handle();

        let args = lang.get_command();
        let child = Command::new(&args[0]).args(&args[1..])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;

        let (sink, stream) = AsyncChildIo::new(child, &handle)?.framed(RpcCodec).split();

        let responses = Rc::new(MsQueue::new());
        let client = RpcClient::new(sink, responses.clone());

        let notifications = Rc::new(MsQueue::new());
        let notifs = notifications.clone();

        let worker = stream.for_each(move |incoming_message| {
                match incoming_message {
                    IncomingMessage::Response(message) => {
                        debug!("pushing a response {:?}", message);
                        responses.push(message);
                        Ok(())
                    }
                    IncomingMessage::Notification(notification) => {
                        debug!("pushing a notification {:?}", notification);
                        notifs.push(notification);
                        Ok(())
                    }
                    _ => Ok(()),
                }
            })
            .map_err(|_| ());

        core.handle().spawn(worker);

        let ls = LanguageServer {
            client: client,
            notifications: notifications,
            core: core,
        };
        Ok(ls)
    }

    pub fn initialize(&self) -> RequestHandle {
        unsafe {
            let pid = libc::getpid();
            let cwd = env::current_dir().unwrap();
            self.client.call(RequestMessage {
                jsonrpc: "2.0".to_string(),
                id: Uuid::new_v4(),
                method: "initialize".to_string(),
                params: json::to_value(ObjectBuilder::new()
                    .insert("processId", pid)
                    .insert("rootPath", cwd)
                    .insert_object("initializationOptions", |builder| builder)
                    .insert_object("capabilities", |builder| builder)
                    .build()),
            })
        }
    }
}
