extern crate env_logger;
#[macro_use] extern crate log;
extern crate tokio_core;
extern crate tokio_language_server_protocol as lib;
extern crate futures;

use lib::{Language, LanguageServer};
use std::process::{Command, Stdio};
// use futures::Future;
use tokio_core::reactor::Core;

struct Golang;

impl Language for Golang {
    fn get_command(&self) -> Vec<String> {
        vec!("langserver-go".to_string())
    }
}

#[test]
fn golang_language_server_starts() {
    let args = Golang.get_command();
    Command::new(&args[0]).args(&args[1..])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
}

#[test]
fn golang_language_server_can_initialize() {
    drop(env_logger::init());

    let mut core = Core::new().unwrap();

    let server = LanguageServer::new(Golang, core.handle()).unwrap();
    let request = server.initialize();
    let response = core.run(request);
    assert!(response.is_ok());
}
