mod credentials;
mod error;

pub use error::Error;
use git2::{build::*, *};
use std::path::{Path, PathBuf};

use self::credentials::create_credentials;

use super::DatabaseUser;

pub struct Database<'a, U: DatabaseUser> {
    path: PathBuf,
    repo: Repository,
    pub user: &'a U,
}

impl<'a, U: DatabaseUser> Database<'a, U> {
    pub fn new(user: &'a U, path: &PathBuf, origin_url: &str) -> Result<Self, Error> {
        if let Some(repo) = Self::open_repo(&path)? {
            Ok(Self {
                path: path.clone(),
                repo,
                user,
            })
        } else {
            Ok(Self {
                repo: Self::clone_repo(origin_url, &path, &user)?,
                path: path.clone(),
                user,
            })
        }
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    pub fn get_username(&self) -> Result<String, Error> {
        if let Some(username) = Config::open_default()?.get_entry("user.name")?.value() {
            Ok(username.into())
        } else {
            Err(git2::Error::new(
                git2::ErrorCode::NotFound,
                git2::ErrorClass::Config,
                "Credentials are not setup correctly, username not found.",
            )
            .into())
        }
    }

    pub fn get_email(&self) -> Result<String, Error> {
        if let Some(email) = Config::open_default()?.get_entry("user.email")?.value() {
            Ok(email.into())
        } else {
            Err(git2::Error::new(
                git2::ErrorCode::NotFound,
                git2::ErrorClass::Config,
                "Credentials are not setup correctly, email not found.",
            )
            .into())
        }
    }

    /// Equivalent to: `git reset --hard origin/main`
    pub fn discard_local(&self) -> Result<(), Error> {
        let origin = self.repo.find_branch("origin/main", BranchType::Remote)?;
        let commit = origin.get().peel_to_commit()?.into_object();

        let checkout = &mut new_checkout_with_progress(self.user);

        self.repo.reset(&commit, ResetType::Hard, Some(checkout))?;
        Ok(())
    }

    // It also moves the head to origin
    pub fn pull(&self) -> Result<Vec<String>, Error> {
        let mut remote = self.repo.find_remote("origin")?;
        let mut fetch_options = new_fetch_options_with_progress(self.user);
        fetch_options.download_tags(git2::AutotagOption::All);
        remote.fetch(&["main"], Some(&mut fetch_options), None)?;

        let fetch_head = self.repo.find_reference("HEAD")?;
        let fetch_commit = self.repo.reference_to_annotated_commit(&fetch_head)?;

        self.merge("main", fetch_commit)
    }

    pub fn push(&self) -> Result<(), Error> {
        let config = git2::Config::open_default()?;

        let mut cbs = RemoteCallbacks::new();
        cbs.credentials(|url, username, _allowed| create_credentials(&config, url, username));

        let mut push_options = PushOptions::new();
        push_options.remote_callbacks(cbs);

        let mut remote = self.repo.find_remote("origin")?;
        remote.push(&["refs/heads/main"], Some(&mut push_options))?;
        Ok(())
    }

    pub fn check_local_credentials(
        &self,
        origin_url: &str,
    ) -> Result<Option<(String, Error)>, Error> {
        // try load config
        let config_res = Config::open_default();
        let Ok(config) = config_res else {
            let msg = "Default configuration not found.".into();
            return if let Err(err) = config_res {
                Ok(Some((msg, err.into())))
            } else {
            Ok(Some((msg, Error::unknown())))
            }
        };

        // try to do a signature
        if let Err(err) = self.repo.signature() {
            if config.get_entry("user.name")?.value().is_none() {
                return Ok(Some(("Missing username.".into(), err.into())));
            }
            if config.get_entry("user.email")?.value().is_none() {
                return Ok(Some(("Missing user email.".into(), err.into())));
            }
            return Ok(Some(("Could not create signature.".into(), err.into())));
        }
        // try to setup a credential
        if let Err(err) = create_credentials(&config, origin_url, None) {
            return Ok(Some(("Could not create credentials".into(), err.into())));
        }

        Ok(None)
    }

    /// Equivelent to do `add .` and `commit -m <message>`
    pub fn commit_all(&self, message: &str) -> Result<(), Error> {
        let mut index = self.repo.index()?;
        let mut found_changes = false;
        let mut error = None;

        if let Err(err) = index.add_all(
            ["*"].iter(),
            IndexAddOption::FORCE,
            Some(&mut |path: &Path, _matched_spec: &[u8]| -> i32 {
                const SKIP: i32 = 1;
                const ADD: i32 = 0;
                const ERROR: i32 = -1;

                match self.repo.status_file(path) {
                    Ok(status) => {
                        if status.is_empty() {
                            SKIP
                        } else {
                            found_changes = true;
                            ADD
                        }
                    }
                    Err(err) => {
                        error = Some(err);
                        ERROR
                    }
                }
            }),
        ) {
            if let Some(err) = error {
                return Err(err.into());
            } else {
                return Err(err.into());
            }
        }

        if found_changes {
            index.write()?;
            let tree = self.repo.find_tree(index.write_tree_to(&self.repo)?)?;
            self.commit(message, &tree, &[&self.repo.head()?.peel_to_commit()?])?;
        }
        Ok(())
    }

    fn commit(&self, msg: &str, tree: &Tree, parents: &[&Commit]) -> Result<(), Error> {
        let s = self.repo.signature()?;
        self.repo.commit(Some("HEAD"), &s, &s, msg, tree, parents)?;
        Ok(())
    }

    fn clone_repo(origin_url: &str, dst: &PathBuf, user: &U) -> Result<Repository, Error> {
        std::fs::create_dir_all(&dst)?;

        // Clean folder for fresh install
        std::fs::remove_dir_all(&dst)?;

        let repo = RepoBuilder::new()
            .fetch_options(new_fetch_options_with_progress(user))
            .with_checkout(new_checkout_with_progress(user))
            .clone(origin_url, dst)?;

        Ok(repo)
    }

    fn open_repo(path: &PathBuf) -> Result<Option<Repository>, Error> {
        match Repository::open(&path) {
            Ok(repo) => {
                if repo.branches(None).map(|b| b.count() == 0).unwrap_or(true) {
                    Ok(None)
                } else {
                    Ok(Some(repo))
                }
            }
            Err(e) => match e.class() {
                git2::ErrorClass::Repository => Ok(None),
                _ => {
                    panic!("Error reading server app folder. {:?}", e);
                }
            },
        }
    }

    fn fast_forward(
        &self,
        lb: &mut git2::Reference,
        rc: &AnnotatedCommit,
    ) -> Result<(), git2::Error> {
        let name = match lb.name() {
            Some(s) => s.to_string(),
            None => String::from_utf8_lossy(lb.name_bytes()).to_string(),
        };
        let msg = format!("Fast-Forward: Setting {} to id: {}", name, rc.id());
        lb.set_target(rc.id(), &msg)?;
        self.repo.set_head(&name)?;
        self.repo.checkout_head(Some(
            // Force is required to make the working directory actually get update
            new_checkout_with_progress(self.user).force(),
        ))?;
        Ok(())
    }

    fn normal_merge(
        &self,
        local: &AnnotatedCommit,
        remote: &AnnotatedCommit,
    ) -> Result<Vec<String>, Error> {
        let local_tree = self.repo.find_commit(local.id())?.tree()?;
        let remote_tree = self.repo.find_commit(remote.id())?.tree()?;
        let ancestor = self
            .repo
            .find_commit(self.repo.merge_base(local.id(), remote.id())?)?
            .tree()?;
        let mut idx = self
            .repo
            .merge_trees(&ancestor, &local_tree, &remote_tree, None)?;

        if idx.has_conflicts() {
            let conflicts = idx.conflicts()?;
            return Ok(conflicts
                .map(|conflict| conflict.expect("Merge conflict resulted in error"))
                .map(|conflict| conflict.our.expect("Merge conflict resulted in error"))
                .map(|conflict| String::from_utf8_lossy(&conflict.path).to_string())
                .collect());
        }
        let result_tree = self.repo.find_tree(idx.write_tree_to(&self.repo)?)?;
        // now create the merge commit
        let msg = format!(
            "Merge: {} into {}",
            remote.refname().unwrap_or(&remote.id().to_string()),
            local.refname().unwrap_or(&local.id().to_string())
        );
        let local_commit = self.repo.find_commit(local.id())?;
        let remote_commit = self.repo.find_commit(remote.id())?;
        // Do our merge commit and set current branch head to that commit.
        self.commit(&msg, &result_tree, &[&local_commit, &remote_commit])?;

        // Set working tree to match head.
        self.repo.checkout_head(None)?;
        Ok(vec![])
    }

    fn merge(
        &self,
        remote_branch: &str,
        fetch_commit: AnnotatedCommit,
    ) -> Result<Vec<String>, Error> {
        let analysis = self.repo.merge_analysis(&[&fetch_commit])?;

        if analysis.0.is_fast_forward() {
            let refname = format!("refs/heads/{}", remote_branch);
            match self.repo.find_reference(&refname) {
                Ok(mut r) => {
                    self.fast_forward(&mut r, &fetch_commit)?;
                }
                Err(_) => {
                    // The branch doesn't exist so just set the reference to the
                    // commit directly. Usually this is because you are pulling
                    // into an empty self.repository.
                    self.repo.reference(
                        &refname,
                        fetch_commit.id(),
                        true,
                        &format!("Setting {} to {}", remote_branch, fetch_commit.id()),
                    )?;
                    self.repo.set_head(&refname)?;
                    self.repo.checkout_head(Some(
                        git2::build::CheckoutBuilder::default()
                            .allow_conflicts(true)
                            .conflict_style_merge(true)
                            .force(),
                    ))?;
                }
            };
            Ok(vec![])
        } else if analysis.0.is_normal() {
            let head_commit = self
                .repo
                .reference_to_annotated_commit(&self.repo.head()?)?;
            let conflicts = self.normal_merge(&head_commit, &fetch_commit)?;
            Ok(conflicts)
        } else {
            Ok(vec![])
        }
    }
}

fn new_fetch_options_with_progress<U: DatabaseUser>(user: &U) -> FetchOptions {
    let mut progress_callback = RemoteCallbacks::new();

    let mut past_progress_ratio = 0.;
    progress_callback.transfer_progress(move |stats| {
        let mut progress_ratio = stats.indexed_objects() as f32 / stats.total_objects() as f32;
        progress_ratio = (1000. * progress_ratio).round() / 1000.;

        if past_progress_ratio != progress_ratio {
            past_progress_ratio = progress_ratio;
            user.report_progress("Downloading".into(), progress_ratio);
        }
        true
    });

    let mut fetch_options = FetchOptions::new();
    fetch_options.remote_callbacks(progress_callback);

    fetch_options
}

fn new_checkout_with_progress<U: DatabaseUser>(user: &U) -> CheckoutBuilder {
    let mut checkout = CheckoutBuilder::new();

    let mut past_progress_ratio = 0.;

    checkout.progress(move |_, curl, total| {
        let mut progress_ratio = curl as f32 / total as f32;
        progress_ratio = (1000. * progress_ratio).round() / 1000.;

        if past_progress_ratio != progress_ratio {
            past_progress_ratio = progress_ratio;
            user.report_progress("Checkout".into(), progress_ratio);
        }
    });

    checkout
}
