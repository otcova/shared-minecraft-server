mod database;

use self::database::{connect_to_database, DatabaseUser};
use super::*;
use std::{sync::mpsc, thread, time::Duration};

/// Used to send scene changes to the frontend
#[derive(Clone)]
struct BackendUser {
    update_scene: mpsc::Sender<Scene>,
    egui_ctx: egui::Context,
}

impl DatabaseUser for BackendUser {
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
}

pub struct Backend {
    scene_recv: mpsc::Receiver<Scene>,
    update_scene: mpsc::Sender<Scene>,
    database_action: mpsc::Sender<database::Action>,
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
            Self::backend_thread(backend_user, database_action_recv);
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

        if let Err(err) = self.database_action.send(database::Action::Lock) {
            self.update_scene
                .send(Scene::fatal_error("Could not send action"))
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

        if let Err(err) = self.database_action.send(database::Action::Unlock) {
            self.update_scene
                .send(Scene::fatal_error("Could not send action"))
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

    fn backend_thread(backend_user: BackendUser, action_recv: mpsc::Receiver<database::Action>) {
        let mut action = database::Action::Unlock;
        loop {
            if let Err(err) = connect_to_database(&backend_user, || {
                match try_pull_until_last(&action_recv) {
                    Received::Some(recv_action) => action = recv_action,
                    Received::Empty => {}
                    Received::ChannelClosed => {
                        backend_user
                            .set_scene(Scene::fatal_error("Backend action channel closed."));
                    }
                };
                action
            }) {
                backend_user.set_scene(Scene::fatal_error(&format!("{}", err)));
                return;
            }
            const COOLDOWN: Duration = Duration::from_secs(5);
            action = action_recv.recv_timeout(COOLDOWN).unwrap_or(action);
        }
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
