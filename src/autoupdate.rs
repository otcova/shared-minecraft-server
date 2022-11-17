use crate::{fetch::*, process::run_detached_process};
use std::{
    env::current_exe,
    fs::{rename, write},
    io,
    process::exit,
};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub const REMOTE_VERSION_ULR: &str =
    "https://raw.githubusercontent.com/otcova/shared-minecraft-server/main/releases/last/version.txt";
pub const REMOTE_APP_ULR: &str =
    "https://raw.githubusercontent.com/otcova/shared-minecraft-server/main/releases/last/Shared Minecraft Server.exe";

/// If there is a new version it will download it and restart the application.
pub fn update() {
    if let Some(remote_version) = fetch_str(REMOTE_VERSION_ULR) {
        if remote_version.as_str() > VERSION {
            let _ = install_new_version();
        }
    }
}

fn install_new_version() -> io::Result<()> {
    if let Some(new_app_data) = fetch_bin(REMOTE_APP_ULR) {
        let exe_path = current_exe()?;
        let mut temp_exe_path = exe_path.clone();
        temp_exe_path.set_file_name(format!(".{}.tmp", env!("CARGO_PKG_NAME")));

        write(&temp_exe_path, new_app_data)?;
        rename(&temp_exe_path, &exe_path)?;

        run_detached_process(&exe_path)?;
        exit(0);
    }
    Ok(())
}
