#[cfg(windows)]
use std::os::windows::process::CommandExt;

use std::ffi::OsStr;
use std::io;
use std::process::{Child, ExitStatus, Stdio};

#[cfg(windows)]
pub fn new_command<S: AsRef<OsStr>>(binary: S) -> std::process::Command {
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    let mut cmd = std::process::Command::new(binary);
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd
}

#[cfg(windows)]

pub fn run_command<S: AsRef<OsStr>>(cmd: S) -> io::Result<Result<String, String>> {
    let out = new_command("powershell")
        .args([OsStr::new("-c"), cmd.as_ref()])
        .output()?;

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

#[cfg(windows)]
pub fn run_detached_process<S: AsRef<OsStr>>(binary: S) -> io::Result<()> {
    use std::ffi::OsString;

    let mut cmd = OsString::from("start '");
    cmd.push(binary);
    cmd.push("'");

    let _ = run_command(cmd)?;
    Ok(())
}
