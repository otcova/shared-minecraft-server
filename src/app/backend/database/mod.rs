mod git;
mod local_files;

use self::git::{Database, Error};
use self::local_files::HOSTER_FILE_LOCAL_PATH;
use super::*;
use crate::app::user;

const SERVER_REPO_URL: &str = "https://github.com/otcova-helper/mc-pasqua";

pub trait DatabaseUser {
    fn report_progress(&self, process_type: String, done_ratio: f32);
    fn set_scene(&self, scene: Scene);
}

fn try_sync_with_origin<U: DatabaseUser, F>(user: &U, on_sync: F) -> Result<(), Error>
where
    F: FnOnce(&Database<U>) -> Result<(), Error>,
{
    let local_files_path = local_files::get_app_folder_path()?;
    let database = Database::new(user, &local_files_path, SERVER_REPO_URL)?;

    if let Some((message, error)) = database.check_local_credentials(SERVER_REPO_URL)? {
        user.set_scene(Scene::Error {
            title: "You need to setup your credentials".into(),
            message,
            details: if error == Error::unknown() {
                "".into()
            } else {
                format!("{}", error)
            },
        });
        return Ok(());
    }

    database.commit_all("before try_sync")?;

    let conflicts = database.pull()?;
    if conflicts.len() > 0 {
        if conflicts.len() == 1 && conflicts[0] == HOSTER_FILE_LOCAL_PATH {
            database.discard_local()?;
            return try_sync_with_origin(user, on_sync);
        } else {
            database.user.set_scene(Scene::RepoConflicts {
                conflicts_count: conflicts.len(),
            });

            return Ok(());
        }
    }

    database.push()?;

    on_sync(&database)
}

#[derive(Copy, Clone)]
pub enum Action {
    /// If the server is locked it will try to unlocked.
    /// If it's already unlocked it will do nothing.
    Unlock,
    /// If it's already locked it will do nothing.
    Lock,
}

pub fn connect_to_database<U: DatabaseUser, F>(user: &U, mut on_sync: F) -> Result<(), Error>
where
    F: FnMut() -> Action,
{
    try_sync_with_origin(user, move |database| {
        let action = on_sync();

        let current_host = local_files::load_current_host(database.path())?;
        let user_id = user::id_from(&database.get_username()?, &database.get_email()?);

        if current_host.as_ref() == Some(&user_id) {
            match action {
                Action::Lock => user.set_scene(Scene::SelfLocked),
                Action::Unlock => {
                    local_files::set_current_host(database.path(), None)?;
                    connect_to_database(user, on_sync)?;
                }
            }
        } else if let Some(host_id) = current_host {
            if let Some(host) = user::parse_id(&host_id) {
                user.set_scene(Scene::SomeoneLocked {
                    host_name: host.username.into(),
                    host_ip: host.ip.into(),
                });
            } else {
                user.set_scene(Scene::SomeoneLocked {
                    host_name: host_id.into(),
                    host_ip: "".into(),
                });
            }
        } else {
            match action {
                Action::Unlock => user.set_scene(Scene::Unlocked),
                Action::Lock => {
                    local_files::set_current_host(database.path(), Some(&user_id))?;
                    connect_to_database(user, on_sync)?;
                }
            }
        }

        Ok(())
    })
}
