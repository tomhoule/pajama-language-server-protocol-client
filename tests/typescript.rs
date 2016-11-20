extern crate tokio_language_server_protocol as lib;

use lib::{Language, LanguageServer};
use std::process::Command;

struct Typescript;

impl Language for Typescript {
    fn get_command(&self) -> Vec<String> {
        vec!("sh".to_string(), "-c".to_string(), "tsserver".to_string())
    }
}

#[test]
fn typescript_language_server_starts() {
    let args = Typescript.get_command();
    Command::new(&args[0]).args(&args[1..]).spawn().unwrap();
}

#[test]
fn typescript_language_server_can_initialize() {
    LanguageServer::new(Typescript).unwrap();
}
