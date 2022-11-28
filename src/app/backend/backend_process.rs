use super::{
    database::{self, connect_to_database, local_files},
    BackendUser, CommandSender,
};
use crate::{app::Scene, ddns, process::stream_command, pull_channel::*};
use std::{
    io::{BufRead, BufReader},
    process::Child,
    sync::{
        mpsc::{channel, Receiver, Sender},
        Arc, Mutex,
    },
    thread,
    time::Duration,
};

#[derive(Copy, Clone)]
pub enum Action {
    Database(database::Action),
    OpenServer(u8),
}

pub struct BackendProcess {
    backend_user: BackendUser,
    action_recv: Receiver<Action>,
    action: Action,
}

impl BackendProcess {
    pub fn start(backend_user: BackendUser) -> Sender<Action> {
        let (action_sender, action_recv) = channel();

        thread::spawn(move || {
            Self {
                backend_user,
                action_recv,
                action: Action::Database(database::Action::Unlock),
            }
            .run_backend_loop();
        });

        action_sender
    }

    fn run_backend_loop(mut self) {
        loop {
            match self.pull_action_channel(Duration::from_secs(5)) {
                Action::Database(_) => self.connect_to_database(),
                Action::OpenServer(ram) => {
                    self.open_server(ram);
                    self.pull_action_channel(Duration::ZERO);
                    self.action = Action::Database(database::Action::Unlock);
                }
            }
        }
    }

    fn connect_to_database(&mut self) {
        let action = match self.pull_action_channel(Duration::ZERO) {
            Action::Database(action) => action,
            Action::OpenServer(_) => database::Action::Lock,
        };
        if let Err(err) = connect_to_database(&self.backend_user, || action.clone()) {
            self.backend_user
                .set_scene(Scene::fatal_error(&format!("{}", err)));
        }
    }

    fn pull_action_channel(&mut self, mut timeout: Duration) -> Action {
        match self.action {
            Action::OpenServer(_) => timeout = Duration::ZERO,
            Action::Database(_) => {}
        };

        match pull_until_last(&self.action_recv, timeout) {
            Received::Some(action) => self.action = action,
            Received::Empty => {}
            Received::ChannelClosed => {
                self.action = Action::Database(database::Action::Unlock);
                self.backend_user
                    .fatal_error("Backend action channel has closed.");
            }
        }
        self.action.clone()
    }

    fn open_server(&self, ram: u8) {
        match local_files::get_app_folder_path() {
            Err(err) => self.backend_user.fatal_error(&format!("{}", err)),
            Ok(server_path) => {
                if let Err(error) = ddns::update() {
                    self.backend_user.set_scene(Scene::Error {
                        title: "Error".into(),
                        message: "Could not update dns.\nContact with a moderator.\n".into(),
                        details: format!("{}", error),
                    });
                    return;
                }

                let start_server_command = format!(
                    r#"cd "{}"; java -Xmx{1}g -Xms{1}g -jar mc_server.jar nogui"#,
                    server_path.display(),
                    ram
                );

                match stream_command("powershell", ["-c", &start_server_command]) {
                    Ok(process) => self.run_server(process),
                    Err(err) => self.backend_user.fatal_error(&format!("{}", err)),
                }
            }
        }
    }

    fn run_server(&self, mut process: Child) {
        let Some(stdout) = process.stdout.take() else {
			self.backend_user.fatal_error("Could not get stdout from the Minecraft Server process.");
			return;
		};

        let Some(stderr) = process.stderr.take() else {
            self.backend_user.fatal_error("Could not get stderr from the Minecraft Server process.");
            return;
        };

        let Some(stdin) = process.stdin.take() else {
            self.backend_user.fatal_error("Could not get stdin from the Minecraft Server process.");
            return;
        };

        let server_output = Arc::new(Mutex::new("".into()));

        self.backend_user.set_scene(Scene::Hosting {
            server_output: server_output.clone(),
            command: "".into(),
            command_sender: CommandSender::new(stdin),
        });

        let stdout_reader = BufReader::new(stdout);

        for line in stdout_reader.lines() {
            if let Ok(line) = line {
                if let Some(line) = filter_chat(&line) {
                    let Ok(mut out) = server_output.lock() else {
                        self.backend_user.fatal_error("Could not lock server output.");
                        return;
                    };

                    *out += &format!("{}\n", line);

                    self.backend_user.request_repaint();
                }
            }
        }

        let stderr_reader = BufReader::new(stderr);

        for line in stderr_reader.lines() {
            println!("> {:?}", line);
        }

        match process.wait() {
            Err(err) => self.backend_user.fatal_error(&format!(
                "Could not launch successfuly the Minecraft Server because of: {:?}",
                err
            )),
            Ok(status) => {
                if !status.success() {
                    self.backend_user.fatal_error(&format!(
                        "Minecraft Server exit with error code: {:?}",
                        status
                    ));
                }
            }
        };
    }
}

fn filter_chat(line: &str) -> Option<String> {
    let line = line.trim();
    if line.len() < 29 {
        return None;
    }
    if &line[0..1] != "[" {
        return None;
    }
    let Ok(_hour) = line[1..3].parse::<u8>() else {
        return None;
    };
    let Ok(_min) = line[4..6].parse::<u8>() else {
        return None;
    };
    let Ok(_sec) = line[7..9].parse::<u8>() else {
        return None;
    };

    if &line[9..15] != " INFO]" {
        return None;
    }

    if &line[15..30] == ": [Not Secure] " {
        Some(format!("{}] {}", &line[0..9], &line[30..]))
    } else if line.ends_with(" joined the game") {
        Some(format!("{}] {}", &line[0..9], &line[17..]))
    } else if line.ends_with(" left the game") {
        Some(format!("{}] {}", &line[0..9], &line[17..]))
    } else {
        println!("{}", line);
        None
    }
}

// [11:57:35 INFO]: [Not Secure] [Server] Hello
// [12:04:19 INFO]: [Not Secure] <Octova> Hello
