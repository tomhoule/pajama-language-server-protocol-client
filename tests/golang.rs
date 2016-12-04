extern crate env_logger;
#[macro_use] extern crate log;
extern crate tokio_language_server_protocol as lib;
extern crate futures;

use lib::{Language, LanguageServer};
use std::process::Command;
// use futures::Future;

struct Golang;

impl Language for Golang {
    fn get_command(&self) -> Vec<String> {
        vec!("langserver-go".to_string())
    }
}

// #[test]
fn golang_language_server_starts() {
    let args = Golang.get_command();
    Command::new(&args[0]).args(&args[1..]).spawn().unwrap();
}

#[test]
fn golang_language_server_can_initialize() {
    drop(env_logger::init());
    let mut server = LanguageServer::new(Golang).unwrap();
    let request = server.initialize();
    let response = server.core.run(request);
    assert!(response.is_ok());
}
