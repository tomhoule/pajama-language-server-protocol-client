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

    #[derive(Deserialize, Debug)]
    pub enum CompletionResult {
        CompletionList(CompletionList),
        CompletionItems(Vec<CompletionItem>),
    }

    #[derive(Deserialize, Debug)]
    pub enum LocationOrLocationList {
        Location(Location),
        Locations(Vec<Location>)
    }
}

pub use language::Language;

use evented_receiver::EventedReceiver;
use std::process::{Command, Stdio};
use error::{Error, Result as CustomResult};
use tokio_core::reactor::{Handle, PollEvented};
use language_server_io::AsyncChildIo;
use client::RpcClient;
use messages::{ServerNotification, Notification, RequestMessage, IncomingMessage, ResponseError};
use tokio_service::Service;
use futures::stream::Stream;
use serde_json as json;
use codec::RpcCodec;
use tokio_core::io::Io;
use futures::Future;
use types::*;
use utils::handle_response;
use serde::{Serialize, Deserialize};

pub trait RpcFuture<R, E>: Future<Item=Result<R, E>, Error=Error> {}
impl<R, E> RpcFuture<R, E> for Future<Item=Result<R, E>, Error=Error> {}

pub struct LanguageServer {
    client: RpcClient,
    pub notifications: Box<Stream<Item = ServerNotification, Error = Error>>,
}

macro_rules! requests {
    ( $( $name:ident: $method:expr, $params:ty, $result:ty, $error:ty, $docstring:expr;)+ )=> {$(
        #[doc=$docstring]
        pub fn $name(&mut self, params: $params) -> impl 'static + Future<Item=Result<$result, ResponseError<$error>>>
        {
            self.call_with_params($method, params)
        }
    )*}
}

macro_rules! client_notifications {
    ( $( $name:ident: $method:expr, $params:ty, $docstring:expr;)+ )=> {$(
        #[doc=$docstring]
        pub fn $name(&self, params: $params) -> impl 'static + Future<Item=()>
        {
            self.notify_with_params($method, params)
        }
    )*}
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

        let worker = stream.map_err(Error::from)
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

    fn call_with_params<'a, REQ, RES, ERR>(&mut self, method: &'static str, params: REQ) -> impl 'a + Future<Item=Result<RES, ERR>, Error=Error>
        where RES: Deserialize + 'static,
              ERR: Deserialize + 'static,
              REQ: Serialize
    {

        self.client.call(RequestMessage::new(method.to_string(), json::to_value(params)))
            .then(|res| handle_response(res?))
    }

    requests!(
        initialize: REQUEST__Initialize, InitializeParams, InitializeResult, InitializeError, "Initializes the server";
        shutdown: REQUEST__Shutdown, (), json::Value, (), "";
        completion: REQUEST__Completion, TextDocumentPositionParams, CompletionResult, (), "";
        resolve_completion: REQUEST__ResolveCompletionItem, CompletionItem, CompletionItem, (), "";
        hover: REQUEST__Hover, TextDocumentPositionParams, Hover, (), "";
        signature_help: REQUEST__SignatureHelp, TextDocumentPositionParams, SignatureHelp, (), "";
        goto_definition: REQUEST__GotoDefinition, TextDocumentPositionParams, LocationOrLocationList, (), "";
        find_references: REQUEST__References, ReferenceParams, Vec<Location>, (), "";
        document_highlights: REQUEST__DocumentHighlight, TextDocumentPositionParams, Vec<DocumentHighlight>, (), "";
        document_symbols: REQUEST__DocumentSymbols, DocumentSymbolParams, Vec<SymbolInformation>, (), "";
        workspace_symbols: REQUEST__WorkspaceSymbols, WorkspaceSymbolParams, Vec<SymbolInformation>, (), "";
        code_action: REQUEST__CodeAction, CodeActionParams, Vec<languageserver_types::Command>, (), "";
        code_lens: REQUEST__CodeLens, CodeLensParams, Vec<CodeLens>, (), "";
        resolve_code_lens: REQUEST__CodeLensResolve, CodeLens, CodeLens, (), "";
        range_formatting: REQUEST__RangeFormatting, DocumentRangeFormattingParams, Vec<TextEdit>, (), "";
        on_type_formatting: REQUEST__OnTypeFormatting, DocumentRangeFormattingParams, Vec<TextEdit>, (), "";
        rename: REQUEST__Rename, RenameParams, WorkspaceEdit, (), "";
    );

    // TODO: DocumentLink

    fn notify_with_params<'a, REQ>(&self, method: &'static str, params: REQ) -> impl 'a + Future<Item=(), Error=Error>
        where REQ: Serialize
    {
        self.client.notify(Notification::new(method.to_string(), json::to_value(params)))
    }

    client_notifications!(
        cancel_request: NOTIFICATION__Cancel, CancelParams, "";
        did_change_configuration: NOTIFICATION__WorkspaceChangeConfiguration, DidChangeConfigurationParams, "";
        did_change_text_document: NOTIFICATION__DidChangeTextDocument, DidChangeTextDocumentParams, "";
        did_change_watched_files: NOTIFICATION__DidChangeWatchedFiles, DidChangeWatchedFilesParams, "";
        did_close_text_document: NOTIFICATION__DidCloseTextDocument, DidCloseTextDocumentParams, "";
        did_open_text_document: NOTIFICATION__DidOpenTextDocument, DidOpenTextDocumentParams, "";
        did_save_text_document: NOTIFICATION__DidSaveTextDocument, DidSaveTextDocumentParams, "";
        exit: NOTIFICATION__Exit, (), "";
    );
}
