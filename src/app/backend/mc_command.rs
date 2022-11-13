use std::io::{self, Write};
use std::process::ChildStdin;

#[derive(Debug)]
pub struct CommandSender {
    sender: ChildStdin,
}

impl CommandSender {
    pub fn new(stdin: ChildStdin) -> Self {
        Self { sender: stdin }
    }
    pub fn send(&mut self, cmd: &str) -> io::Result<()> {
        writeln!(self.sender, "say {}", cmd)
    }

    pub fn send_stop(&mut self) -> io::Result<()> {
        writeln!(self.sender, "stop")
    }
}
