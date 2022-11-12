use git2::{Config, Cred, Error};
use std::io::Write;
use std::process::Stdio;

use crate::process::new_command;

/// git2::Cred::credential_helper opens a console on windows.
/// This is a silent alteranive.
pub fn create_credentials(
    config: &Config,
    url: &str,
    username: Option<&str>,
) -> Result<Cred, Error> {
    match CredentialHelper::new(url)
        .config(config)
        .username(username)
        .execute()
    {
        Some((username, password)) => Cred::userpass_plaintext(&username, &password),
        None => Err(Error::from_str(
            "failed to acquire username/password from local configuration",
        )),
    }
}

// -------------------------------------------------------------
// --------- The following code is mainly from git2 ------------
// -------------------------------------------------------------

macro_rules! debug {
    ($($arg:tt)+) => {};
}

/// Management of the gitcredentials(7) interface.
struct CredentialHelper {
    username: Option<String>,
    protocol: Option<String>,
    host: Option<String>,
    port: Option<u16>,
    path: Option<String>,
    url: String,
    commands: Vec<String>,
}

impl CredentialHelper {
    /// Create a new credential helper object which will be used to probe git's
    /// local credential configuration.
    ///
    /// The url specified is the namespace on which this will query credentials.
    /// Invalid urls are currently ignored.
    fn new(url: &str) -> CredentialHelper {
        let mut ret = CredentialHelper {
            protocol: None,
            host: None,
            port: None,
            path: None,
            username: None,
            url: url.to_string(),
            commands: Vec::new(),
        };

        // Parse out the (protocol, host) if one is available
        if let Ok(url) = url::Url::parse(url) {
            if let Some(url::Host::Domain(s)) = url.host() {
                ret.host = Some(s.to_string());
            }
            ret.port = url.port();
            ret.protocol = Some(url.scheme().to_string());
        }
        ret
    }

    /// Set the username that this credential helper will query with.
    ///
    /// By default the username is `None`.
    fn username(&mut self, username: Option<&str>) -> &mut CredentialHelper {
        self.username = username.map(|s| s.to_string());
        self
    }

    /// Query the specified configuration object to discover commands to
    /// execute, usernames to query, etc.
    fn config(&mut self, config: &Config) -> &mut CredentialHelper {
        // Figure out the configured username/helper program.
        //
        // see http://git-scm.com/docs/gitcredentials.html#_configuration_options
        if self.username.is_none() {
            self.config_username(config);
        }
        self.config_helper(config);
        self.config_use_http_path(config);
        self
    }

    // Configure the queried username from `config`
    fn config_username(&mut self, config: &Config) {
        let key = self.exact_key("username");
        self.username = config
            .get_string(&key)
            .ok()
            .or_else(|| {
                self.url_key("username")
                    .and_then(|s| config.get_string(&s).ok())
            })
            .or_else(|| config.get_string("credential.username").ok())
    }

    // Discover all `helper` directives from `config`
    fn config_helper(&mut self, config: &Config) {
        let exact = config.get_string(&self.exact_key("helper"));
        self.add_command(exact.as_ref().ok().map(|s| &s[..]));
        if let Some(key) = self.url_key("helper") {
            let url = config.get_string(&key);
            self.add_command(url.as_ref().ok().map(|s| &s[..]));
        }
        let global = config.get_string("credential.helper");
        self.add_command(global.as_ref().ok().map(|s| &s[..]));
    }

    // Discover `useHttpPath` from `config`
    fn config_use_http_path(&mut self, config: &Config) {
        let mut use_http_path = false;
        if let Some(value) = config.get_bool(&self.exact_key("useHttpPath")).ok() {
            use_http_path = value;
        } else if let Some(value) = self
            .url_key("useHttpPath")
            .and_then(|key| config.get_bool(&key).ok())
        {
            use_http_path = value;
        } else if let Some(value) = config.get_bool("credential.useHttpPath").ok() {
            use_http_path = value;
        }

        if use_http_path {
            if let Ok(url) = url::Url::parse(&self.url) {
                let path = url.path();
                // Url::parse always includes a leading slash for rooted URLs, while git does not.
                self.path = Some(path.strip_prefix('/').unwrap_or(path).to_string());
            }
        }
    }

    // Add a `helper` configured command to the list of commands to execute.
    //
    // see https://www.kernel.org/pub/software/scm/git/docs/technical
    //                           /api-credentials.html#_credential_helpers
    fn add_command(&mut self, cmd: Option<&str>) {
        let cmd = match cmd {
            Some("") | None => return,
            Some(s) => s,
        };

        if cmd.starts_with('!') {
            self.commands.push(cmd[1..].to_string());
        } else if cmd.contains("/") || cmd.contains("\\") {
            self.commands.push(cmd.to_string());
        } else {
            self.commands.push(format!("git credential-{}", cmd));
        }
    }

    fn exact_key(&self, name: &str) -> String {
        format!("credential.{}.{}", self.url, name)
    }

    fn url_key(&self, name: &str) -> Option<String> {
        match (&self.host, &self.protocol) {
            (&Some(ref host), &Some(ref protocol)) => {
                Some(format!("credential.{}://{}.{}", protocol, host, name))
            }
            _ => None,
        }
    }

    /// Execute this helper, attempting to discover a username/password pair.
    ///
    /// All I/O errors are ignored, (to match git behavior), and this function
    /// only succeeds if both a username and a password were found
    fn execute(&self) -> Option<(String, String)> {
        let mut username = self.username.clone();
        let mut password = None;
        for cmd in &self.commands {
            let (u, p) = self.execute_cmd(cmd, &username);
            if u.is_some() && username.is_none() {
                username = u;
            }
            if p.is_some() && password.is_none() {
                password = p;
            }
            if username.is_some() && password.is_some() {
                break;
            }
        }

        match (username, password) {
            (Some(u), Some(p)) => Some((u, p)),
            _ => None,
        }
    }

    // Execute the given `cmd`, providing the appropriate variables on stdin and
    // then afterwards parsing the output into the username/password on stdout.
    fn execute_cmd(
        &self,
        cmd: &str,
        username: &Option<String>,
    ) -> (Option<String>, Option<String>) {
        macro_rules! my_try( ($e:expr) => (
            match $e {
                Ok(e) => e,
                Err(e) => {
                    debug!("{} failed with {}", stringify!($e), e);
                    return (None, None)
                }
            }
        ) );

        // It looks like the `cmd` specification is typically bourne-shell-like
        // syntax, so try that first. If that fails, though, we may be on a
        // Windows machine for example where `sh` isn't actually available by
        // default. Most credential helper configurations though are pretty
        // simple (aka one or two space-separated strings) so also try to invoke
        // the process directly.
        //
        // If that fails then it's up to the user to put `sh` in path and make
        // sure it works.
        let mut c = new_command("sh");
        c.arg("-c")
            .arg(&format!("{} get", cmd))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        debug!("executing credential helper {:?}", c);
        let mut p = match c.spawn() {
            Ok(p) => p,
            Err(e) => {
                debug!("`sh` failed to spawn: {}", e);
                let mut parts = cmd.split_whitespace();
                let mut c = new_command(parts.next().unwrap());
                for arg in parts {
                    c.arg(arg);
                }
                c.arg("get")
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped());
                debug!("executing credential helper {:?}", c);
                match c.spawn() {
                    Ok(p) => p,
                    Err(e) => {
                        debug!("fallback of {:?} failed with {}", cmd, e);
                        return (None, None);
                    }
                }
            }
        };

        // Ignore write errors as the command may not actually be listening for
        // stdin
        {
            let stdin = p.stdin.as_mut().unwrap();
            if let Some(ref p) = self.protocol {
                let _ = writeln!(stdin, "protocol={}", p);
            }
            if let Some(ref p) = self.host {
                if let Some(ref p2) = self.port {
                    let _ = writeln!(stdin, "host={}:{}", p, p2);
                } else {
                    let _ = writeln!(stdin, "host={}", p);
                }
            }
            if let Some(ref p) = self.path {
                let _ = writeln!(stdin, "path={}", p);
            }
            if let Some(ref p) = *username {
                let _ = writeln!(stdin, "username={}", p);
            }
        }
        let output = my_try!(p.wait_with_output());
        if !output.status.success() {
            debug!(
                "credential helper failed: {}\nstdout ---\n{}\nstderr ---\n{}",
                output.status,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            return (None, None);
        }
        debug!(
            "credential helper stderr ---\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        self.parse_output(output.stdout)
    }

    // Parse the output of a command into the username/password found
    fn parse_output(&self, output: Vec<u8>) -> (Option<String>, Option<String>) {
        // Parse the output of the command, looking for username/password
        let mut username = None;
        let mut password = None;
        for line in output.split(|t| *t == b'\n') {
            let mut parts = line.splitn(2, |t| *t == b'=');
            let key = parts.next().unwrap();
            let value = match parts.next() {
                Some(s) => s,
                None => {
                    debug!("ignoring output line: {}", String::from_utf8_lossy(line));
                    continue;
                }
            };
            let value = match String::from_utf8(value.to_vec()) {
                Ok(s) => s,
                Err(..) => continue,
            };
            match key {
                b"username" => username = Some(value),
                b"password" => password = Some(value),
                _ => {}
            }
        }
        (username, password)
    }
}
