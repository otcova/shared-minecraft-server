use super::*;

#[derive(Debug)]
pub struct App {
    scene: Scene,
    window_size: Option<Vec2>,
    username: String,
    ram: u8,
}

impl App {
    pub fn new(cc: &eframe::CreationContext) -> Self {
        Self {
            scene: Scene::Main,
            window_size: None,
            username: local_storage::get_str!(cc.storage, "username", "".into()),
            ram: local_storage::get_num!(cc.storage, "ram", 2),
        }
    }
    fn resize_window(&mut self, frame: &mut eframe::Frame, new_size: Vec2) {
        if self.window_size.is_none() || self.window_size.unwrap() != new_size {
            frame.set_window_size(new_size);
            self.window_size = Some(new_size);
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, win_frame: &mut eframe::Frame) {
        Self::setup_fonts(ctx);

        egui::CentralPanel::default().show(ctx, |ui| {
            let frame = egui::Frame::default().inner_margin(egui::style::Margin::same(10.));
            let frame_response = frame.show(ui, |ui| self.draw_scene(ui, win_frame)).response;

            let size = match self.scene {
                Scene::Hosting { .. } => vec2(700., 446.),
                _ => vec2(300., frame_response.rect.height() + 20.),
            };
            self.resize_window(win_frame, size);
        });
    }
}

impl App {
    fn setup_fonts(ctx: &egui::Context) {
        use FontFamily::Proportional;
        let mut style = (*ctx.style()).clone();
        style.text_styles = [
            (TextStyle::Heading, FontId::new(25.0, Proportional)),
            (TextStyle::Body, FontId::new(17.0, Proportional)),
            (TextStyle::Monospace, FontId::new(16.0, Proportional)),
            (TextStyle::Button, FontId::new(20.0, Proportional)),
        ]
        .into();
        ctx.set_style(style);
    }

    fn set_style(ui: &mut egui::Ui) {
        ui.spacing_mut().button_padding = vec2(10., 10.);
        ui.spacing_mut().item_spacing = vec2(10., 10.);

        if ui.style().visuals.dark_mode {
            ui.style_mut().visuals.override_text_color = Some(Color32::from_rgb(220, 220, 220));
        } else {
            ui.style_mut().visuals.override_text_color = Some(Color32::from_rgb(0, 0, 0));
        }

        ui.style_mut().visuals.text_cursor_width = 1.;
    }

    fn draw_scene(&mut self, ui: &mut egui::Ui, win_frame: &mut eframe::Frame) {
        Self::set_style(ui);

        match &mut self.scene {
            Scene::Main => {
                ui.heading("Server Offline");
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("Username:");
                    if ui.text_edit_singleline(&mut self.username).changed() {
                        local_storage::set!(win_frame, "username", &self.username);
                    }
                });

                ui.set_enabled(self.username.len() > 0);

                if ui.button("Lock Server").clicked() {
                    self.lock_server();
                }
            }
            Scene::SomeoneLocked => {
                ui.heading("Server Locked");
                ui.separator();
                ui.label("Host name: Octova");
                ui.label("Host ip: 122.261.101.231");
            }
            Scene::SelfLocked => {
                ui.heading("You own the Server");
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label(format!("Ram: {}GB", self.ram));
                    let slider = Slider::new(&mut self.ram, 1..=6).show_value(false);
                    if ui.add(slider).changed() {
                        local_storage::set!(win_frame, "ram", self.ram);
                    }
                });

                if ui.button("Start Server").clicked() {
                    // self.start_server();
                }
            }
            Scene::Hosting {
                server_output,
                command,
            } => {
                ui.heading("You are hosting");
                ui.separator();
                Frame::default().show(ui, |ui| {
                    ui.spacing_mut().item_spacing = vec2(0., 0.);
                    ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .stick_to_bottom(true)
                        .max_height(300.)
                        .show(ui, |ui| {
                            ui.label(server_output.clone());
                        });

                    ui.add(TextEdit::singleline(command).desired_width(f32::INFINITY));
                });

                if ui.button("Close Server").clicked() {
                    // self.scene = Scene::Hosting;
                }
            }
            Scene::Uploading => {
                ui.heading("Uploading Changes");
                ui.label("fetching: 45%");
            }
            Scene::Downloading => {
                ui.heading("Downloading Changes");
                ui.label("fetching: 45%");
            }
        }
    }
}
