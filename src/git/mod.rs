mod credentials;
mod status_reporter;

use crate::error::Error;
use git2::{build::RepoBuilder, *};
pub use status_reporter::StatusReporter;
use status_reporter::*;
use std::path::{Path, PathBuf};

use self::credentials::create_credentials;

pub struct Git<R: StatusReporter> {
    path: PathBuf,
    repo: Repository,
    reporter: GitStatusReporter<R>,
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
        if let Some(repo) = Self::open(path.as_ref())? {
            Ok(Self {
                path: path.as_ref().into(),
                repo,
                reporter: reporter.into(),
            })
        } else {
            Self::clone(reporter, path, origin_url)
        }
    }

    fn open<P: AsRef<Path>>(path: P) -> Result<Option<Repository>, Error> {
        match Repository::open(&path) {
            Ok(repo) => {
                if repo.branches(None).map(|b| b.count() == 0).unwrap_or(true) {
                    Ok(None)
                } else {
                    Ok(Some(repo))
                }
            }
            Err(e) => match e.code() {
                ErrorCode::NotFound => Ok(None),
                _ => {
                    panic!("Error reading server app folder. {:?}", e);
                }
            },
        }
    }

    fn clone<P: AsRef<Path>>(reporter: R, path: P, origin_url: &str) -> Result<Self, Error> {
        let reporter: GitStatusReporter<R> = reporter.into();

        std::fs::create_dir_all(path.as_ref())?;

        // Clean folder content for fresh install
        std::fs::remove_dir_all(path.as_ref())?;

        let repo = RepoBuilder::new()
            .fetch_options(reporter.new_fetch_options())
            .with_checkout(reporter.new_checkout())
            .clone(origin_url, path.as_ref())?;

        Ok(Self {
            path: path.as_ref().into(),
            repo,
            reporter,
        })
    }

    pub fn check_credentials(
        &self,
        origin_url: &str,
    ) -> Result<Result<(), (&'static str, Error)>, Error> {
        // try load config
        let config_res = Config::open_default();
        let Ok(config) = config_res else {
            let msg = "Default configuration not found.";
            return if let Err(err) = config_res {
                Ok(Err((msg, err.into())))
            } else {
            Ok(Err((msg, Error::unknown())))
            }
        };

        // try to do a signature
        if let Err(err) = self.repo.signature() {
            if config.get_entry("user.name")?.value().is_none() {
                return Ok(Err(("Missing username.", err.into())));
            }
            if config.get_entry("user.email")?.value().is_none() {
                return Ok(Err(("Missing user email.", err.into())));
            }
            return Ok(Err(("Could not create signature.", err.into())));
        }

        // try to setup a credential
        if let Err(err) = create_credentials(&config, origin_url, None) {
            return Ok(Err(("Could not create credentials", err.into())));
        }

        Ok(Ok(()))
    }

    /// If there aren't changes, it will not commit.
    pub fn commit_all(&self, message: &str) -> Result<(), Error> {
        let mut index = self.repo.index()?;
        let mut found_changes = false;

        index.add_all(
            ["*"].iter(),
            IndexAddOption::FORCE,
            Some(&mut |path: &Path, _matched_spec: &[u8]| -> i32 {
                found_changes = true;
                self.reporter.status_change("Listing changes", Some(0.));
                0
            }),
        )?;

        if found_changes {
            index.write()?;
            let tree = self.repo.find_tree(index.write_tree_to(&self.repo)?)?;
            self.commit(message, &tree, &[&self.repo.head()?.peel_to_commit()?])?;
        }
        Ok(())
    }

    fn commit(&self, msg: &str, tree: &Tree, parents: &[&Commit]) -> Result<(), Error> {
        let s = self.repo.signature()?;
        self.reporter.status_change("Packing changes", Some(0.));
        self.repo.commit(Some("HEAD"), &s, &s, msg, tree, parents)?;
        Ok(())
    }

    pub fn push(&self) -> Result<(), Error> {
        let mut push_opts = self.reporter.new_push_options()?;
        let mut remote = self.repo.find_remote("origin")?;
        remote.push(&["refs/heads/main"], Some(&mut push_opts))?;
        Ok(())
    }

    /// Equivelent to: `reset --hard origin/main`
    pub fn reset_hard_to_origin(&self) -> Result<(), Error> {
        let origin = self.repo.find_branch("origin/main", BranchType::Remote)?;
        let commit = origin.get().peel_to_commit()?.into_object();

        let mut checkout = self.reporter.new_checkout();
        self.repo
            .reset(&commit, ResetType::Hard, Some(&mut checkout))?;

        Ok(())
    }

    pub fn pull(&self) -> Result<MergeStatus, Error> {
        let mut remote = self.repo.find_remote("origin")?;
        let mut fetch_options = self.reporter.new_fetch_options();
        fetch_options.download_tags(git2::AutotagOption::All);
        remote.fetch(&["main"], Some(&mut fetch_options), None)?;

        let fetch_head = self.repo.find_reference("HEAD")?;
        let fetch_commit = self.repo.reference_to_annotated_commit(&fetch_head)?;

        self.merge("main", fetch_commit)
    }

    fn merge(
        &self,
        remote_branch: &str,
        fetch_commit: AnnotatedCommit,
    ) -> Result<MergeStatus, Error> {
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
            Ok(MergeStatus::Ok)
        } else if analysis.0.is_normal() {
            let head = self.repo.head()?;
            let head_commit = self.repo.reference_to_annotated_commit(&head)?;

            let local = self.repo.find_commit(head_commit.id())?.tree()?;
            let remote = self.repo.find_commit(fetch_commit.id())?.tree()?;
            let merge_base = self.repo.merge_base(head_commit.id(), fetch_commit.id())?;
            let ancestor = self.repo.find_commit(merge_base)?.tree()?;

            let index = self.repo.merge_trees(&ancestor, &local, &remote, None)?;

            if index.has_conflicts() {
                Ok(MergeStatus::Conflicts(
                    index
                        .conflicts()?
                        .flatten()
                        .map(|conflict| conflict.our)
                        .flatten()
                        .map(|conflict| String::from_utf8_lossy(&conflict.path).to_string())
                        .collect(),
                ))
            } else {
                Ok(MergeStatus::Conflicts(vec![]))
            }
        } else if analysis.0.is_up_to_date() {
            Ok(MergeStatus::Ok)
        } else {
            Err(Error::from_str(format!(
                "Could not merge. Merge analysis code: {}",
                analysis.0.bits()
            )))
        }
    }

    fn fast_forward(&self, lb: &mut git2::Reference, rc: &AnnotatedCommit) -> Result<(), Error> {
        let name = match lb.name() {
            Some(s) => s.to_string(),
            None => String::from_utf8_lossy(lb.name_bytes()).to_string(),
        };
        let msg = format!("Fast-Forward: Setting {} to id: {}", name, rc.id());
        lb.set_target(rc.id(), &msg)?;
        self.repo.set_head(&name)?;

        let mut checkout = self.reporter.new_checkout();
        self.repo.checkout_head(Some(checkout.force()))?;
        Ok(())
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
            "Credentials are not setup correctly, username not found.",
        )
        .into())
    }
}

pub fn get_email() -> Result<String, Error> {
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
