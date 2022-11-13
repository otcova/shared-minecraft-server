use std::ffi::OsStr;
use std::io;
use std::os::windows::process::CommandExt;
use std::process::{Child, ExitStatus, Stdio};

const CREATE_NO_WINDOW: u32 = 0x08000000;

#[cfg(windows)]
pub fn new_command<S: AsRef<OsStr>>(binary: S) -> std::process::Command {
    let mut cmd = std::process::Command::new(binary);
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd
}

#[cfg(windows)]
pub fn run_command(cmd: &str) -> io::Result<Result<String, String>> {
    let out = new_command("powershell").args(["-c", cmd]).output()?;

    Ok(if ExitStatus::success(&out.status) {
        Ok(String::from_utf8_lossy(&out.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).to_string())
    })
}

pub fn stream_command<S, I>(binary: S, args: I) -> io::Result<Child>
where
    S: AsRef<OsStr>,
    I: IntoIterator<Item = S>,
{
    new_command(binary)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::piped())
        .spawn()
}
