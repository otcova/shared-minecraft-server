use git2::{Cred, Error};

pub fn create_credentials() -> Result<Cred, Error> {
    Cred::userpass_plaintext("Octova-Handle", "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")
}
