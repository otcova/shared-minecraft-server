use super::*;
use git2::build::{CheckoutBuilder, RepoBuilder};
use git2::*;
use std::path::PathBuf;
use std::process::Command;

const SERVER_REPO_LINK: &str = "https://github.com/otcova-helper/mc-pasqua";
const SERVER_NAME: &str = "Pasqua";

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

pub fn clone_server_repository(server_folder: &PathBuf, app: &BackendApp) -> Repository {
    // Clean folder for fresh install
    std::fs::remove_dir_all(&server_folder)
        .expect("Could not delete server folder when reinstalling server files");

    let mut progress = 0.;
    app.set_scene(Scene::Loading {
        title: "Downloading",
        progress,
    });

    let mut progress_callback = RemoteCallbacks::new();
    let mut checkout = CheckoutBuilder::new();

    progress_callback.transfer_progress(move |stats| {
        let current_progress = stats.indexed_objects() as f32 / stats.total_objects() as f32;
        if current_progress != progress {
            progress = current_progress;
            app.set_scene(Scene::Loading {
                title: "Downloading",
                progress,
            });
        }
        true
    });

    checkout.progress(move |_path, curl, total| {
        let current_progress = curl as f32 / total as f32;
        if current_progress != progress {
            progress = current_progress;
            app.set_scene(Scene::Loading {
                title: "Checkout",
                progress,
            });
        }
    });

    let mut fetch_options = FetchOptions::new();
    fetch_options.remote_callbacks(progress_callback);

    match RepoBuilder::new()
        .fetch_options(fetch_options)
        .with_checkout(checkout)
        .clone(SERVER_REPO_LINK, server_folder)
    {
        Ok(repo) => repo,
        Err(e) => panic!(
            "Could not clone server repo '{}'. {:?}",
            SERVER_REPO_LINK, e
        ),
    }
}

pub fn get_server_repository(app: &BackendApp) -> Repository {
    let server_folder = get_app_folder_path().join(SERVER_NAME);
    std::fs::create_dir_all(&server_folder).expect("Could not create server folder");

    match Repository::open(&server_folder) {
        Ok(repo) => {
            // If no branches are found, reinstall repo
            let branches = repo.branches(None);
            if branches.map(|b| b.count() == 0).unwrap_or(true) {
                clone_server_repository(&server_folder, app)
            } else {
                repo
            }
        }
        Err(e) => match e.class() {
            git2::ErrorClass::Repository => clone_server_repository(&server_folder, app),
            _ => {
                panic!("Error reading server app folder. {:?}", e);
            }
        },
    }
}

pub fn lock_server(repo: Repository, userid: String) {
    
}