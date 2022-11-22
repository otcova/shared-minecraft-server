pub mod local_files;

use self::local_files::HOSTER_FILE_LOCAL_PATH;
use super::*;
use crate::app::user;
use crate::error::Error;
use crate::git::{Git, MergeStatus, StatusReporter};

const SERVER_REPO_URL: &str = "https://github.com/otcova-helper/mc-pasqua";

impl StatusReporter for &BackendUser {
    fn status_change(&self, operation: &'static str, progress: Option<f32>) {
        self.report_progress(operation.into(), progress.unwrap_or(0.))
    }
}

fn try_sync_with_origin<F>(user: &BackendUser, on_sync: F) -> Result<(), Error>
where
    F: FnOnce(&Git<&BackendUser>) -> Result<(), Error>,
{
    let local_files_path = local_files::get_app_folder_path()?;
    let database = Git::<&BackendUser>::new(user, &local_files_path, SERVER_REPO_URL)?;

    database.commit_all("before try_sync")?;

    match database.pull()? {
        MergeStatus::Conflicts(conflicts) => {
            if conflicts.len() == 1 && conflicts[0] == HOSTER_FILE_LOCAL_PATH {
                database.reset_hard_to_origin()?;
                try_sync_with_origin(user, on_sync)
            } else {
                user.set_scene(Scene::RepoConflicts {
                    conflicts_count: conflicts.len(),
                });

                Ok(())
            }
        }
        MergeStatus::Ok => {
            database.push()?;
            on_sync(&database)
        }
    }
}

#[derive(Copy, Clone)]
pub enum Action {
    /// If the server is locked it will try to unlocked.
    /// If it's already unlocked it will do nothing.
    Unlock,
    /// If it's already locked it will do nothing.
    Lock,
}

pub fn connect_to_database<F>(user: &BackendUser, mut on_sync: F) -> Result<(), Error>
where
    F: FnMut() -> Action,
{
    try_sync_with_origin(user, move |database| {
        let action = on_sync();

        let current_host = local_files::load_current_host(database.work_dir())?;
        let user_id = user::id();

        if current_host.as_ref() == Some(&user_id) {
            match action {
                Action::Lock => user.set_scene(Scene::SelfLocked),
                Action::Unlock => {
                    local_files::set_current_host(database.work_dir(), None)?;
                    connect_to_database(user, on_sync)?;
                }
            }
        } else if let Some(host_id) = current_host {
            user.set_scene(Scene::SomeoneLocked { host_id });
        } else {
            match action {
                Action::Unlock => user.set_scene(Scene::Unlocked),
                Action::Lock => {
                    local_files::set_current_host(database.work_dir(), Some(&user_id))?;
                    connect_to_database(user, on_sync)?;
                }
            }
        }

        Ok(())
    })
}
