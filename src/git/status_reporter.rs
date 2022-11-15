use std::ops::Deref;

use git2::{build::CheckoutBuilder, FetchOptions, PushOptions, RemoteCallbacks};

use crate::error::Error;

use super::credentials::create_credentials;

pub trait StatusReporter {
    /// Progress goes from 0 to 1
    fn status_change(&self, operation: &'static str, progress: Option<f32>);
}

pub struct GitStatusReporter<R: StatusReporter> {
    reporter: R,
}

impl<R: StatusReporter> From<R> for GitStatusReporter<R> {
    fn from(value: R) -> Self {
        Self::new(value)
    }
}

impl<R: StatusReporter> Deref for GitStatusReporter<R> {
	type Target = R;
    fn deref(&self) -> &Self::Target {
		&self.reporter
	}
}

impl<R: StatusReporter> GitStatusReporter<R> {
    pub fn new(reporter: R) -> Self {
        Self { reporter }
    }

    pub fn new_checkout(&self) -> CheckoutBuilder {
        let mut checkout = CheckoutBuilder::new();

        checkout.progress(|_, curl, total| {
            const OPERATION: &'static str = "Updating files";
            if curl == 0 {
                self.reporter.status_change(OPERATION, None);
            } else {
                let progress = curl as f32 / total as f32;
                self.reporter.status_change(OPERATION, Some(progress));
            }
        });

        checkout
    }

    pub fn new_fetch_options(&self) -> FetchOptions {
        let mut fetch_options = FetchOptions::new();
        fetch_options.remote_callbacks(self.new_remote_callbacks("Downloading"));
        fetch_options
    }

    pub fn new_push_options(&self) -> Result<PushOptions, Error> {
        let mut cbs = self.new_remote_callbacks("Uploading");
        
        let config = git2::Config::open_default()?;
        cbs.credentials(move |url, username, _| create_credentials(&config, url, username));

        let mut push_options = PushOptions::new();
        push_options.remote_callbacks(cbs);
        Ok(push_options)
    }

    fn new_remote_callbacks(&self, operation: &'static str) -> RemoteCallbacks {
        let mut progress_callbacks = RemoteCallbacks::new();

        progress_callbacks.transfer_progress(move |stats| {
            if stats.indexed_objects() == 0 {
                self.reporter.status_change(operation, None);
            } else {
                let progress = stats.indexed_objects() as f32 / stats.total_objects() as f32;
                self.reporter.status_change(operation, Some(progress));
            }
            true
        });

        progress_callbacks
    }
}
