use std::io;
use std::os::windows::process::CommandExt;
use std::process::ExitStatus;
const CREATE_NO_WINDOW: u32 = 0x08000000;

#[cfg(windows)]
pub fn new_command(program: &str) -> std::process::Command {
    let mut cmd = std::process::Command::new(program);
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
