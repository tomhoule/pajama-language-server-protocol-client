extern crate env_logger;
#[macro_use] extern crate log;
extern crate tokio_language_server_protocol as lib;
extern crate futures;

use lib::{Language, LanguageServer};
// use std::process::Command;
use futures::Future;

struct Typescript;

impl Language for Typescript {
    fn get_command(&self) -> Vec<String> {
        vec!("sh".to_string(), "-c".to_string(), "tsserver".to_string())
    }
}

// #[test]
// fn typescript_language_server_starts() {
//     let args = Typescript.get_command();
//     Command::new(&args[0]).args(&args[1..]).spawn().unwrap();
// }

#[test]
fn typescript_language_server_can_initialize() {
    drop(env_logger::init());
    let mut server = LanguageServer::new(Typescript).unwrap();
    let response = server.initialize();
    server.core.handle().spawn(response.then(|_| {
        Err(())
    }));
    loop {
        server.core.turn(None)
    }
}
