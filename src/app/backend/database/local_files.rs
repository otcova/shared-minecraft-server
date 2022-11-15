use crate::process;
use std::io;
use std::path::PathBuf;

#[cfg(windows)]
fn create_hidden_folder(path: &str, folder_name: &str) -> io::Result<()> {
    if let Err(output) = process::run_command(&format!(
        "cd \"{}\"; mkdir \"{}\"; attrib +h +s \"{1}\"",
        path, folder_name
    ))? {
        Err(io::Error::new(io::ErrorKind::Other, output))
    } else {
        Ok(())
    }
}

#[cfg(windows)]
pub fn get_app_folder_path() -> io::Result<PathBuf> {
    const APP_FOLDER_NAME: &str = "Octova - Shared Minecraft Server";
    const SERVER_NAME: &str = "Pasqua";

    let Some(base_dirs) = directories::BaseDirs::new() else {
        return Err(io::Error::new(io::ErrorKind::NotFound, "Could not found APP_DATA directory"));
    };

    let app_data_path = base_dirs.data_dir();
    let app_folder = app_data_path.join(APP_FOLDER_NAME);

    if !app_folder.exists() {
        let Some(app_data_path) = app_data_path .to_str() else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "APP_DATA directory has invalid unicode characters")
            );
        };
        create_hidden_folder(app_data_path, APP_FOLDER_NAME)?;
    }

    let server_folder = app_folder.join(SERVER_NAME);
    std::fs::create_dir_all(&server_folder)?;
    Ok(server_folder)
}

pub const HOSTER_FILE_LOCAL_PATH: &str = "hoster.txt";
pub fn set_current_host(path: &PathBuf, host_id: Option<&str>) -> io::Result<()> {
    let file_path = path.join(HOSTER_FILE_LOCAL_PATH);
    match host_id {
        Some(id) => std::fs::write(file_path, id),
        None => match std::fs::remove_file(file_path) {
            Ok(()) => Ok(()),
            Err(ref err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(err) => Err(err),
        },
    }
}

pub fn load_current_host(path: &PathBuf) -> io::Result<Option<String>> {
    let path = path.join(HOSTER_FILE_LOCAL_PATH);

    if path.is_file() {
        let content = std::fs::read(path)?;
        let host_id = String::from_utf8_lossy(&content).to_string();

        if host_id.len() == 0 {
            Ok(None)
        } else {
            Ok(Some(host_id))
        }
    } else {
        Ok(None)
    }
}
