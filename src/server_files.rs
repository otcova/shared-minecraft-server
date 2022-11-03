use git2::*;

use std::path::PathBuf;
use std::process::Command;

#[cfg(windows)]
fn create_hidden_folder(path: &str, folder_name: &str) -> std::io::Result<()> {
    let output = Command::new("powershell")
        .args([
            "-c",
            &format!(
                "cd \"{}\"; mkdir \"{}\"; attrib +h +s \"{1}\"",
                path, folder_name
            ),
        ])
        .output()?;
    format!("[cmd] {:?}", output);
    Ok(())
}

#[cfg(windows)]
fn get_app_folder_path() -> PathBuf {
    const APP_FOLDER_NAME: &str = "Octova - Shared Minecraft Server";
    let base_dirs = directories::BaseDirs::new().expect("Could not found APP_DATA directory");
    let app_data_path = base_dirs.data_dir();
    let app_folder = app_data_path.join(APP_FOLDER_NAME);

    if !app_folder.exists() {
        create_hidden_folder(
            app_data_path
                .to_str()
                .expect("APP_DATA path has invalid UTF-8 characters"),
            APP_FOLDER_NAME,
        )
        .expect("Could not create APP_DATA/.. folder");
    }

    app_folder
}

pub fn get_server_repository() -> Repository {
    const SERVER_REPO_LINK: &str = "https://github.com/otcova/mc-shared-server";
    const SERVER_NAME: &str = "Pasqua";

    let server_folder = get_app_folder_path().join(SERVER_NAME);
    std::fs::create_dir_all(&server_folder).expect("Could not create server folder");

    match Repository::open(&server_folder) {
        Ok(repo) => repo,
        Err(e) => match e.class() {
            git2::ErrorClass::Repository => match Repository::clone(SERVER_REPO_LINK, server_folder) {
                Ok(repo) => repo,
                Err(e) => panic!("Could not clone server repo {:?}", e),
            },
            _ => {
                panic!("Error reading server app folder. {:?}", e);
            }
        },
    }
}
