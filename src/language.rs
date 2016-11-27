pub trait Language {
    /// Return a list of command arguments to be spawned.
    fn get_command(&self) -> Vec<String>;
}
