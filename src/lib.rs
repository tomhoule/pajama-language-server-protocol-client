#![feature(proc_macro)]
#![feature(field_init_shorthand)]
#![feature(conservative_impl_trait)]

#[macro_use]
extern crate chomp;
extern crate futures;
extern crate languageserver_types;
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

mod client;
mod codec;
mod dispatcher;
mod error;
mod evented_receiver;
mod language;
mod language_server_io;
mod message_parser;
mod messages;
mod utils;

pub mod types {
    pub use languageserver_types::*;
}

pub use language::Language;

use evented_receiver::EventedReceiver;
use std::process::{Command, Stdio};
use error::{Error, Result as CustomResult};
use tokio_core::reactor::{Handle, PollEvented};
use language_server_io::AsyncChildIo;
use client::RpcClient;
use messages::{Notification, RequestMessage, IncomingMessage};
use tokio_service::Service;
use futures::stream::Stream;
use serde_json as json;
use codec::RpcCodec;
use tokio_core::io::Io;
use futures::Future;
use languageserver_types::*;
use utils::handle_response;

type Response<R, E> = Box<Future<Item=Result<R, E>, Error=Error>>;

pub struct LanguageServer {
    client: RpcClient,
    pub notifications: Box<Stream<Item = Notification, Error = Error>>,
}

impl LanguageServer {
    pub fn new<L: Language>(lang: L, handle: Handle) -> CustomResult<Self> {
        let args = lang.get_command();
        let child = Command::new(&args[0]).args(&args[1..])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;

        let (sink, stream) = AsyncChildIo::new(child, &handle)?.framed(RpcCodec).split();

        let (responses_sender, responses_receiver) = mio::channel::channel();
        let responses = EventedReceiver::new(PollEvented::new(responses_receiver, &handle)?);
        let client = RpcClient::new(sink, responses);

        let (notifications_sender, notifications_receiver) = mio::channel::channel();
        let notifications = EventedReceiver::new(PollEvented::new(notifications_receiver,
                                                                  &handle)?);

        let worker = stream.map_err(|err| Error::from(err))
            .for_each(move |incoming_message| {
                match incoming_message {
                    IncomingMessage::Response(message) => {
                        debug!("pushing a response {:?}", message);
                        responses_sender.send(message)?;
                        Ok(())
                    }
                    IncomingMessage::Notification(notification) => {
                        debug!("pushing a notification {:?}", notification);
                        notifications_sender.send(notification)?;
                        Ok(())
                    }
                    _ => Ok(()),
                }
            })
            .map_err(|_| ());

        handle.spawn(worker);

        let ls = LanguageServer {
            client: client,
            notifications: Box::new(notifications),
        };
        Ok(ls)
    }

    pub fn initialize(&self, params: InitializeParams) -> Response<InitializeResult, InitializeError> {
        let h = self.client.call(RequestMessage::new("initialize".to_string(), json::to_value(params)));
        Box::new(h.then(|res| {
            handle_response(res?)
        }))
    }
}
