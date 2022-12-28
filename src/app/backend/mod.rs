mod backend_process;
mod database;
mod mc_command;

use crate::pull_channel::{pull_until_last, Received};

use super::*;
use backend_process::*;
pub use mc_command::*;
use std::{sync::mpsc, time::Duration};

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
    action_sender: mpsc::Sender<Action>,
}

impl Backend {
    pub fn new(egui_ctx: &egui::Context) -> Self {
        let (update_scene, scene_recv) = mpsc::channel();

        let backend_user = BackendUser {
            update_scene: update_scene.clone(),
            egui_ctx: egui_ctx.clone(),
        };

        let action_sender = BackendProcess::start_thread(backend_user);

        Self {
            scene_recv,
            update_scene,
            action_sender,
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
            .action_sender
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
            .action_sender
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

        if let Err(err) = self.action_sender.send(Action::OpenServer(ram)) {
            let err = format!("Error on send action to database: {}", err);
            self.update_scene
                .send(Scene::fatal_error(&err))
                .expect("Could not update scene");
        }
    }

    /// Call this function to check for scene updates
    pub fn update_scene(&self) -> Option<Scene> {
        match pull_until_last(&self.scene_recv, Duration::ZERO) {
            Received::Some(new_scene) => Some(new_scene),
            Received::Empty => None,
            Received::ChannelClosed => Some(Scene::fatal_error("Backend channel has been closed.")),
        }
    }
}
