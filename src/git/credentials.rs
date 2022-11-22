use git2::{Cred, Error};

pub fn create_credentials() -> Result<Cred, Error> {
    let header = String::from("ghp");
    Cred::userpass_plaintext("Octova-Handle", &(header + &env!("PUBLIC_GITHUB_TOKEN_BODY")))
}
