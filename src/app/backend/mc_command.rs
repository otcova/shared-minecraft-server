use std::io::{self, Write};
use std::process::ChildStdin;

#[derive(Debug)]
pub struct CommandSender {
    sender: ChildStdin,
    cooldown: u8,
}

impl CommandSender {
    pub fn new(stdin: ChildStdin) -> Self {
        Self {
            sender: stdin,
            cooldown: 5,
        }
    }
    pub fn send(&mut self, cmd: &str) -> io::Result<()> {
        writeln!(self.sender, "say {}", cmd)
    }

    pub fn request_tps(&mut self) -> io::Result<()> {
        if self.cooldown <= 0 {
            self.cooldown = 4;
            writeln!(self.sender, "tps")
        } else {
            self.cooldown -= 1;
            Ok(())
        }
    }

    pub fn send_stop(&mut self) -> io::Result<()> {
        writeln!(self.sender, "stop")
    }
}
