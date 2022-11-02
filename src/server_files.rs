use git2::*;

use std::path::PathBuf;
use std::process::Command;

#[cfg(windows)]
fn create_hidden_folder(path: &str, folder_name: &str) -> std::io::Result<()> {
    Command::new("powershell")
        .args([
            "-C",
            &format!(
                "cd \"{}\"; mkdir \"{}\"; attrib +h +s \"{1}\"",
                path, folder_name
            ),
        ])
        .output()?;
    Ok(())
}

#[cfg(windows)]
fn get_app_folder_path() -> PathBuf {
    const APP_FOLDER_NAME: &str = "Octova - Shared Minecraft Server";
    let base_dirs = directories::BaseDirs::new().expect("Could not found APP_DATA directory");
    let app_folder_path = base_dirs.data_dir();

    create_hidden_folder(
        app_folder_path
            .to_str()
            .expect("APP_DATA path has invalid UTF-8 characters"),
        APP_FOLDER_NAME,
    )
    .expect("Could not create APP_DATA/.. folder");

    app_folder_path.join(APP_FOLDER_NAME)
}
