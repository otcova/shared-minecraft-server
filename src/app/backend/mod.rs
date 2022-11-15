mod database;
mod mc_command;

use crate::process::stream_command;

use self::database::{connect_to_database, local_files};
use super::*;
pub use mc_command::*;
use std::{
    io::{BufRead, BufReader},
    process::Child,
    sync::mpsc,
    thread,
    time::Duration,
};

/// Used to send scene changes to the frontend
#[derive(Clone)]
pub struct BackendUser {
    update_scene: mpsc::Sender<Scene>,
    egui_ctx: egui::Context,
}

impl BackendUser {
    fn report_progress(&self, process_type: String, done_ratio: f32) {
        self.set_scene(Scene::Loading {
            title: process_type,
            progress: done_ratio,
        });
    }
    fn set_scene(&self, new_scene: Scene) {
        self.update_scene
            .send(new_scene)
            .expect("Could not update scene");
        self.egui_ctx.request_repaint();
    }
    fn fatal_error(&self, details: &str) {
        self.set_scene(Scene::fatal_error(details));
    }

    fn request_repaint(&self) {
        self.egui_ctx.request_repaint();
    }
}

pub struct Backend {
    scene_recv: mpsc::Receiver<Scene>,
    update_scene: mpsc::Sender<Scene>,
    database_action: mpsc::Sender<Action>,
}

#[derive(Copy, Clone)]
enum Action {
    Database(database::Action),
    OpenServer(u8),
}

impl Backend {
    pub fn new(egui_ctx: &egui::Context) -> Self {
        let (update_scene, scene_recv) = mpsc::channel();
        let (database_action, database_action_recv) = mpsc::channel();

        let backend_user = BackendUser {
            update_scene: update_scene.clone(),
            egui_ctx: egui_ctx.clone(),
        };

        thread::spawn(move || {
            Self::backend_process(backend_user, database_action_recv);
        });

        Self {
            scene_recv,
            update_scene,
            database_action,
        }
    }

    pub fn lock_server(&self) {
        self.update_scene
            .send(Scene::Loading {
                title: "Locking...".into(),
                progress: 0.,
            })
            .expect("Could not update scene");

        if let Err(err) = self
            .database_action
            .send(Action::Database(database::Action::Lock))
        {
            let err = format!("Error on send action to database: {}", err);
            self.update_scene
                .send(Scene::fatal_error(&err))
                .expect("Could not update scene");
        }
    }

    pub fn unlock_server(&self) {
        self.update_scene
            .send(Scene::Loading {
                title: "Unlocking...".into(),
                progress: 0.,
            })
            .expect("Could not update scene");

        if let Err(err) = self
            .database_action
            .send(Action::Database(database::Action::Unlock))
        {
            let err = format!("Error on send action to database: {}", err);
            self.update_scene
                .send(Scene::fatal_error(&err))
                .expect("Could not update scene");
        }
    }

    pub fn start_server(&self, ram: u8) {
        self.update_scene
            .send(Scene::Loading {
                title: "Launching Minecraft Server...".into(),
                progress: 0.,
            })
            .expect("Could not update scene");

        if let Err(err) = self.database_action.send(Action::OpenServer(ram)) {
            let err = format!("Error on send action to database: {}", err);
            self.update_scene
                .send(Scene::fatal_error(&err))
                .expect("Could not update scene");
        }
    }

    /// Call this function to check for scene updates
    pub fn update_scene(&self) -> Option<Scene> {
        match try_pull_until_last(&self.scene_recv) {
            Received::Some(new_scene) => Some(new_scene),
            Received::Empty => None,
            Received::ChannelClosed => Some(Scene::fatal_error("Backend channel has been closed.")),
        }
    }

    fn backend_process(backend_user: BackendUser, action_recv: mpsc::Receiver<Action>) {
        let mut action = Action::Database(database::Action::Unlock);
        loop {
            match action {
                Action::Database(_) => {
                    Self::connect_to_database(&backend_user, || {
                        match try_pull_until_last(&action_recv) {
                            Received::Some(recv_action) => action = recv_action,
                            Received::Empty => {}
                            Received::ChannelClosed => {
                                backend_user.fatal_error("Backend action channel closed.");
                            }
                        };
                        action.clone()
                    });
                    
                    match action {
                        Action::OpenServer(_) => match try_pull_until_last(&action_recv) {
                            Received::Some(recv_action) => action = recv_action,
                            Received::Empty => {}
                            Received::ChannelClosed => {
                                backend_user.fatal_error("Backend action channel closed.");
                            }
                        },
                        _ => {
                            const COOLDOWN: Duration = Duration::from_secs(5);
                            action = action_recv.recv_timeout(COOLDOWN).unwrap_or(action);
                        }
                    }
                }
                Action::OpenServer(ram) => {
                    println!("S");
                    Self::open_server(&backend_user, ram);
                    let _ = try_pull_until_last(&action_recv);
                    action = Action::Database(database::Action::Unlock);
                }
            }
        }
    }

    fn connect_to_database<F>(backend_user: &BackendUser, mut on_sync: F)
    where
        F: FnMut() -> Action,
    {
        if let Err(err) = connect_to_database(&backend_user, || match on_sync() {
            Action::Database(action) => action,
            Action::OpenServer(ram) => database::Action::Lock,
        }) {
            backend_user.set_scene(Scene::fatal_error(&format!("{}", err)));
        }
    }

    fn open_server(backend_user: &BackendUser, ram: u8) {
        match local_files::get_app_folder_path() {
            Err(err) => backend_user.set_scene(Scene::fatal_error(&format!("{}", err))),
            Ok(server_path) => {
                let start_server_command = format!(
                    r#"cd "{}"; java -Xmx{1}g -Xms{1}g -jar mc_server.jar nogui"#,
                    server_path.display(),
                    ram
                );

                match stream_command("powershell", ["-c", &start_server_command]) {
                    Ok(process) => Self::run_server(backend_user, process),
                    Err(err) => backend_user.fatal_error(&format!("{}", err)),
                }
            }
        }
    }

    fn run_server(backend_user: &BackendUser, mut process: Child) {
        let Some(stderr) = process.stderr.take() else {
            backend_user.fatal_error("Could not get stderr from the Minecraft Server process.");
            return;
        };

        let Some(stdout) = process.stdout.take() else {
            backend_user.fatal_error("Could not get stdout from the Minecraft Server process.");
            return;
        };

        let Some(stdin) = process.stdin.take() else {
            backend_user.fatal_error("Could not get stdin from the Minecraft Server process.");
            return;
        };

        let server_output = Arc::new(Mutex::new("".into()));

        backend_user.set_scene(Scene::Hosting {
            server_output: server_output.clone(),
            command: "".into(),
            command_sender: CommandSender::new(stdin),
        });

        let stdout_reader = BufReader::new(stdout);

        for line in stdout_reader.lines() {
            let Ok(mut out) = server_output.lock() else {
                backend_user.fatal_error("Could not lock server output.");
                return;
            };
            match line {
                Ok(line) => *out += &format!("{}\n", line),
                Err(line) => *out += &format!("[ERROR] {}", line),
            }

            backend_user.request_repaint();
        }

        let stderr_reader = BufReader::new(stderr);

        for line in stderr_reader.lines() {
            println!("> {:?}", line);
        }

        match process.wait() {
            Err(err) => backend_user.fatal_error(&format!(
                "Could not launch successfuly the Minecraft Server because of: {:?}",
                err
            )),
            Ok(status) => {
                if !status.success() {
                    backend_user.fatal_error(&format!(
                        "Minecraft Server exit with error code: {:?}",
                        status
                    ));
                }
            }
        };
    }
}

enum Received<T> {
    Some(T),
    Empty,
    ChannelClosed,
}
/// Returns the most recent received item and discards all the rest.
fn try_pull_until_last<T>(receiver: &mpsc::Receiver<T>) -> Received<T> {
    let mut data = Received::Empty;

    loop {
        match receiver.try_recv() {
            Ok(recv_data) => data = Received::Some(recv_data),
            Err(mpsc::TryRecvError::Empty) => return data,
            Err(mpsc::TryRecvError::Disconnected) => match &data {
                Received::Empty => return Received::ChannelClosed,
                _ => return data,
            },
        }
    }
}
