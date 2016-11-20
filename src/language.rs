use std::process::Command;
use std::io;

pub trait Language {
    /// Return a Command to be spawned
    fn get_command(&self) -> Vec<String>;
}
