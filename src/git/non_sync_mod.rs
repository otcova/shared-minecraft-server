mod credentials;
mod status_reporter;

use crate::error::Error;
use git2::Config;
pub use status_reporter::StatusReporter;
use std::path::{Path, PathBuf};

pub struct Git<R: StatusReporter> {
    path: PathBuf,
    reporter: R,
}

pub enum MergeStatus {
    Ok,
    // Merge has been aborted because it could not fast forward.
    Conflicts(Vec<String>),
}

impl<R: StatusReporter> Git<R> {
    /// It will open a repo if exists and clone from url if not.
    /// If needed it will create the path.
    pub fn new<P: AsRef<Path>>(reporter: R, path: P, origin_url: &str) -> Result<Self, Error> {
        Ok(Self {
            reporter,
            path: path.as_ref().into(),
        })
    }

    /// If there aren't changes, it will not commit.
    pub fn commit_all(&self, message: &str) -> Result<(), Error> {
        Ok(())
    }

    pub fn push(&self) -> Result<(), Error> {
        Ok(())
    }

    /// Equivelent to: `reset --hard origin/main`
    pub fn reset_hard_to_origin(&self) -> Result<(), Error> {
        Ok(())
    }

    pub fn pull(&self) -> Result<MergeStatus, Error> {
        Ok(MergeStatus::Ok)
    }

    pub fn work_dir(&self) -> &PathBuf {
        &self.path
    }
}

pub fn get_username() -> Result<String, Error> {
    if let Some(username) = Config::open_default()?.get_entry("user.name")?.value() {
        Ok(username.into())
    } else {
        Err(git2::Error::new(
            git2::ErrorCode::NotFound,
            git2::ErrorClass::Config,
            "Credentials are not setup properly, username not found.",
        )
        .into())
    }
}
