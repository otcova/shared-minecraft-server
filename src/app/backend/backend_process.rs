use super::{
    database::{self, connect_to_database, local_files},
    BackendUser, CommandSender,
};
use crate::{
    app::Scene, ddns, process::stream_command, pull_channel::*, verify_signature::verify_signature,
};
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
        let mut cooldown = Duration::ZERO;
        loop {
            match self.pull_action_channel(cooldown) {
                Action::Database(_) => self.connect_to_database(),
                Action::OpenServer(ram) => {
                    self.open_server(ram);
                    self.pull_action_channel(Duration::ZERO);
                    self.action = Action::Database(database::Action::Unlock);
                }
            }
            cooldown = Duration::from_secs(5);
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
                if !verify_signature(server_path.join("mc_server.jar")) {
                    self.backend_user.set_scene(Scene::Error {
                        title: "Error".into(),
                        message: "The signature is not valid!\n\
                            There is a signature to prevent other hosters from injecting viruses in the server.\
                            If the signature doesn't validate, it means that the files have been modified without authorization.\n\
                            Contact with a moderator!".into(),
                        details: "".into(),
                    });
                    return;
                }

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

        let ui_chat = Arc::new(Mutex::new("".into()));
        let ui_players = Arc::new(Mutex::new(vec![]));
        let ui_tps = Arc::new(Mutex::new(20.));

        self.backend_user.set_scene(Scene::Hosting {
            chat: ui_chat.clone(),
            players: ui_players.clone(),
            tps: ui_tps.clone(),
            command: "".into(),
            command_sender: CommandSender::new(stdin),
        });

        let stdout_reader = BufReader::new(stdout);

        for line in stdout_reader.lines() {
            if let Ok(line) = line {
                match parse_console_log(&line) {
                    ConsoleLog::Chat { msg } => {
                        let Ok(mut out) = ui_chat.lock() else {
                            self.backend_user.fatal_error("Could not lock server output.");
                            return;
                        };

                        *out += &msg;

                        self.backend_user.request_repaint();
                    }
                    ConsoleLog::Joined { player_name, msg } => {
                        let Ok(mut out) = ui_chat.lock() else {
                            self.backend_user.fatal_error("Could not lock server output.");
                            return;
                        };

                        *out += &msg;

                        if let Ok(mut players_list) = ui_players.lock() {
                            players_list.push(player_name);
                        }

                        self.backend_user.request_repaint();
                    }
                    ConsoleLog::Left { player_name, msg } => {
                        let Ok(mut out) = ui_chat.lock() else {
                            self.backend_user.fatal_error("Could not lock server output.");
                            return;
                        };

                        *out += &msg;

                        if let Ok(mut players_list) = ui_players.lock() {
                            if let Some(player_index) =
                                players_list.iter().position(|name| *name == player_name)
                            {
                                players_list.remove(player_index);
                            }
                        }

                        self.backend_user.request_repaint();
                    }
                    ConsoleLog::Tps { tps } => {
                        let Ok(mut ui_tps) = ui_tps.lock() else {
                            self.backend_user.fatal_error("Could not lock server output.");
                            return;
                        };

                        *ui_tps = tps;

                        self.backend_user.request_repaint();
                    }
                    ConsoleLog::Other => {}
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

enum ConsoleLog {
    Chat { msg: String },
    Joined { player_name: String, msg: String },
    Left { player_name: String, msg: String },
    Tps { tps: f32 },
    Other,
}

fn parse_console_log(line: &str) -> ConsoleLog {
    let line = line.trim();
    if line.len() < 31 {
        return ConsoleLog::Other;
    }
    if &line[0..1] != "[" {
        return ConsoleLog::Other;
    }
    let Ok(_hour) = line[1..3].parse::<u8>() else {
        return ConsoleLog::Other;
    };
    let Ok(_min) = line[4..6].parse::<u8>() else {
        return ConsoleLog::Other;
    };
    let Ok(_sec) = line[7..9].parse::<u8>() else {
        return ConsoleLog::Other;
    };

    if &line[9..15] != " INFO]" {
        return ConsoleLog::Other;
    }

    if line.len() > 50 && &line[15..44] == ": TPS from last 1m, 5m, 15m: " {
        match line[44..].split_once(", ") {
            Some((tps_str, _)) => match tps_str.parse::<f32>() {
                Ok(tps) => ConsoleLog::Tps { tps },
                Err(_) => ConsoleLog::Other,
            },
            None => ConsoleLog::Other,
        }
    } else if &line[15..30] == ": [Not Secure] " {
        ConsoleLog::Chat {
            msg: format!("{}] {}\n", &line[0..9], &line[30..]),
        }
    } else if line.ends_with(" joined the game") {
        ConsoleLog::Joined {
            player_name: line[17..line.len() - 16].to_string(),
            msg: format!("{}] {}\n", &line[0..9], &line[17..]),
        }
    } else if line.ends_with(" left the game") {
        ConsoleLog::Left {
            player_name: line[17..line.len() - 14].to_string(),
            msg: format!("{}] {}\n", &line[0..9], &line[17..]),
        }
    } else {
        ConsoleLog::Other
    }
}
