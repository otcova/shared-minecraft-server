mod backend;
mod local_storage;
mod user;

use self::backend::{Backend, CommandSender};
use super::*;
use eframe::egui::style::Margin;
use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub enum Scene {
    Unlocked,
    SomeoneLocked {
        host_name: String,
        host_ip: String,
    },
    SelfLocked,
    Hosting {
        server_output: Arc<Mutex<String>>,
        command: String,
        command_sender: CommandSender,
    },
    Loading {
        title: String,
        progress: f32,
    },
    RepoConflicts {
        conflicts_count: usize,
    },
    Error {
        title: String,
        message: String,
        details: String,
    },
}

impl Scene {
    fn fatal_error(details: &str) -> Scene {
        Scene::Error {
            title: "Error".into(),
            message: "Contact with a moderator.".into(),
            details: String::from(details),
        }
    }
}

pub struct App {
    scene: Scene,
    ram: u8,
    backend: Backend,
    try_close: bool,
}

impl App {
    pub fn new(cc: &eframe::CreationContext) -> Self {
        Self::setup_fonts(&cc.egui_ctx);

        Self {
            scene: Scene::Loading {
                title: "Connecting".into(),
                progress: 0.,
            },
            backend: Backend::new(&cc.egui_ctx),
            ram: local_storage::get_num!(cc.storage, "ram", 2),
            try_close: false,
        }
    }

    /// return true if the window has resized
    fn resize_window(&mut self, frame: &mut eframe::Frame, new_size: Vec2) {
        if frame.info().window_info.size != new_size {
            frame.set_window_size(new_size);
        }
    }

    fn can_close(&self) -> bool {
        match &self.scene {
            Scene::RepoConflicts { .. }
            | Scene::Error { .. }
            | Scene::Unlocked
            | Scene::SomeoneLocked { .. } => true,
            _ => false,
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, win_frame: &mut eframe::Frame) {
        match &self.scene {
            Scene::Error { .. } => {}
            Scene::RepoConflicts { .. } => {}
            _ => {
                if let Some(new_scene) = self.backend.update_scene() {
                    self.scene = new_scene;
                }
            }
        }

        if self.try_close {
            if self.can_close() {
                win_frame.close()
            } else {
                self.backend.unlock_server()
            }
        }

        let margin = 18.;
        let central_panel_frame = Frame::none()
            .fill(ctx.style().visuals.window_fill())
            .inner_margin(Margin::same(margin));

        CentralPanel::default()
            .frame(central_panel_frame)
            .show(ctx, |ui| {
                let frame = Frame::default().show(ui, |ui| self.draw_scene(ui, win_frame));
                let auto_height = frame.response.rect.height() + margin * 2.;

                let size = match &self.scene {
                    Scene::Hosting { .. } => vec2(700., auto_height),
                    Scene::RepoConflicts { .. } => vec2(740., auto_height),
                    _ => vec2(300., auto_height),
                };

                self.resize_window(win_frame, size);
            });
    }

    fn on_close_event(&mut self) -> bool {
        match &mut self.scene {
            Scene::Hosting { command_sender, .. } => {
                if let Err(err) = command_sender.send_stop() {
                    self.scene = Scene::Loading {
                        title: "Closing Minecraft Server".into(),
                        progress: 0.,
                    };
                }
            }
            _ => {}
        };
        self.backend.unlock_server();
        self.try_close = true;
        self.can_close()
    }
}

impl App {
    fn setup_fonts(ctx: &egui::Context) {
        use FontFamily::Proportional;
        let mut style = (*ctx.style()).clone();

        style.spacing.text_edit_width = f32::INFINITY;
        style.text_styles = [
            (TextStyle::Heading, FontId::new(25., Proportional)),
            (TextStyle::Body, FontId::new(17., Proportional)),
            (TextStyle::Monospace, FontId::new(16., Proportional)),
            (TextStyle::Button, FontId::new(20., Proportional)),
            (TextStyle::Small, FontId::new(15., Proportional)),
        ]
        .into();

        style.spacing.button_padding = vec2(10., 10.);
        style.spacing.item_spacing = vec2(10., 10.);

        if style.visuals.dark_mode {
            style.visuals.override_text_color = Some(Color32::from_rgb(220, 220, 220));
        } else {
            style.visuals.override_text_color = Some(Color32::from_rgb(0, 0, 0));
        }

        style.visuals.text_cursor_width = 1.;

        ctx.set_style(style);
    }

    fn set_font_size(ui: &mut egui::Ui, text_style: TextStyle, size: f32) {
        ui.style_mut()
            .text_styles
            .iter_mut()
            .find(|(style, _)| **style == text_style)
            .map(|(_, font)| font.size = size);
    }

    fn draw_scene(&mut self, ui: &mut egui::Ui, win_frame: &mut eframe::Frame) {
        match &mut self.scene {
            Scene::Unlocked => {
                ui.heading("Server Offline");
                ui.separator();
                if ui.button("Lock Server").clicked() {
                    self.backend.lock_server();
                }
            }
            Scene::SomeoneLocked { host_name, host_ip } => {
                ui.heading("Server Locked");
                ui.separator();
                if host_ip.len() == 0 {
                    ui.label(format!("Host: {}", host_name));
                } else {
                    ui.label(format!("Host name: {}", host_name));
                    ui.label(format!("Host ip: {}", host_ip));
                }
            }
            Scene::SelfLocked => {
                ui.heading("You have the Power");
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label(format!("Ram: {}GB", self.ram));
                    let slider = Slider::new(&mut self.ram, 1..=6).show_value(false);
                    if ui.add(slider).changed() {
                        local_storage::set!(win_frame, "ram", self.ram);
                    }
                });

                if ui.button("Start Server").clicked() {
                    self.backend.start_server(self.ram);
                }
            }
            Scene::Hosting {
                server_output,
                command,
                command_sender,
            } => {
                ui.horizontal_top(|ui| {
                    ui.heading("You are hosting on:");

                    ui.vertical(|ui| {
                        let pub_ip = public_ip::get().expect("Could not get public ip");
                        let font_size = 22.;
                        Self::set_font_size(ui, TextStyle::Body, font_size);
                        ui.spacing_mut().item_spacing.y = 0.;
                        ui.allocate_space(Vec2::new(0., 25. - font_size));

                        let link = ui.link(&pub_ip).on_hover_text("Copy to clipboard");
                        if link.clicked() {
                            ui.output().copied_text = pub_ip;
                        }
                    });
                });

                ui.separator();

                let Ok(text) = server_output.lock() else {
                    self.scene = Scene::fatal_error("Backend panicked while holding a lock.");
                    ui.ctx().request_repaint();
                    return;
                };

                Frame::default().show(ui, |ui| {
                    ui.spacing_mut().item_spacing = vec2(0., 0.);

                    ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .stick_to_bottom(true)
                        .max_height(300.)
                        .show(ui, |ui| {
                            ui.label(&*text);
                        });

                    let input = ui.text_edit_singleline(command);
                    if input.lost_focus() && ui.input().key_down(Key::Enter) {
                        let _ = command_sender.send(command);
                        *command = "".into();
                        input.request_focus();
                    }
                });

                if ui.button("Close Server").clicked() {
                    let _ = command_sender.send_stop();
                }
            }
            Scene::Loading { title, progress } => {
                ui.heading(title);
                if *progress > 0. {
                    ui.horizontal(|ui| {
                        ui.label(format!("{:.1}%", *progress * 100.));
                        ui.add(ProgressBar::new(*progress));
                    });
                }
            }
            Scene::RepoConflicts { conflicts_count } => {
                ui.heading(&format!("{} Conflicts!", conflicts_count));
                ui.separator();
                ui.label(REPO_CONFLICT_EXPLENATION.replace(" ", "  "));

                let mut button = egui::Button::new("Delete all local progress");
                if ui.style().visuals.dark_mode {
                    button = button.fill(Color32::from_rgb(130, 10, 10));
                } else {
                    button = button.fill(Color32::from_rgb(255, 150, 150));
                }
                ui.add(button);
            }
            Scene::Error {
                title,
                message,
                details,
            } => {
                ui.heading(title);
                ui.separator();
                ui.label(&*message);
                if details.len() > 0 {
                    ui.indent("details", |ui| {
                        ui.small("Details:");
                        ui.small(&*details);
                    });
                }
            }
        }
    }
}

const REPO_CONFLICT_EXPLENATION: &str = "
You hosted and modifyed the world to version B.
 - Local timeline: A (world before your hosting) -- your hosting --> B (world after your hosting)

But your world didn't upload correctly to the database,
so the database never received version B.

Later another hoster started hosting from the database,
crated the version C and uploaded to the database.
 - Database timeline: A (world before your hosting) -- hosting --> C (world after hosting)

Currently there are three options:
1) Delete world B and keep playing with the current database timeline.
2) Delete world C and upload to the database the world B.
3) Do magic... maybe.

If you whant to proceede with the option 1 use the red button. Otherwise contact with a Moderator.
";
