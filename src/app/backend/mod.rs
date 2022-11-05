mod server_repo;

use super::*;
use server_repo::get_server_repository;
use std::{sync::mpsc, thread};

/// Used to send scene changes to the frontend
#[derive(Clone)]
pub struct BackendApp {
    sender: mpsc::Sender<Scene>,
    egui_ctx: egui::Context,
}

impl BackendApp {
    fn set_scene(&self, new_scene: Scene) {
        let _ = self.sender.send(new_scene);
        self.egui_ctx.request_repaint();
    }
}

impl App {
    pub fn pull_data_from_backend(&mut self) {
        if let Some(receiver) = self.backend_receiver.as_mut() {
            loop {
                match receiver.try_recv() {
                    Ok(new_scene) => self.scene = new_scene,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        self.backend_receiver = None;
                        break;
                    }
                    Err(_) => break,
                }
            }
        }
    }

    /// It makes sure that only a single backend process is running.
    fn start_backend_process<F>(&mut self, process: F)
    where
        F: FnOnce(BackendApp) + Send + 'static,
    {
        if self.backend_receiver.is_none() {
            let (tx, rx) = mpsc::channel();
            self.backend_receiver = Some(rx);

            let backend_handle = BackendApp {
                sender: tx,
                egui_ctx: self.egui_ctx.clone(),
            };

            thread::spawn(move || {
                process(backend_handle);
            });
        }
    }

    pub fn lock_server(&mut self) {
        self.start_backend_process(move |app| {
            let _repo = get_server_repository(&app);
        });
    }
}
