use egui::Ui;
use rfd::FileDialog;
use std::{
    path::PathBuf,
    sync::mpsc::{channel, Receiver},
};

use crate::project::Project;

const APP_KEY: &str = "ccaccounting";

#[derive(Default, Debug, serde::Deserialize, serde::Serialize)]
pub struct App {
    input_file: Option<PathBuf>,
    #[serde(skip)]
    project: Option<Project>,
    #[serde(skip)]
    wait_for_file: Option<Receiver<Option<PathBuf>>>,
}

impl App {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let base: Self = if let Some(storage) = cc.storage {
            let old_state = eframe::get_value(storage, APP_KEY);
            // tracing::info!("old state loaded: {:?}", old_state);
            old_state.unwrap_or_default()
        } else {
            Default::default()
        };

        let project = if let Some(input_file) = &base.input_file {
            Project::new(input_file.clone()).ok()
        } else {
            None
        };

        Self {
            project,
            wait_for_file: None,
            ..base
        }
    }

    fn draw_menu(&mut self, ui: &mut Ui, frame: &mut eframe::Frame) {
        egui::menu::bar(ui, |ui| {
            ui.menu_button("File", |ui| {
                if let Some(project) = self.project.as_mut() {
                    project.populate_menu(ui);

                    if ui.button("Close Project").clicked() {
                        ui.close_menu();
                        self.project = None;
                        self.input_file = None;
                    }
                } else if self.wait_for_file.is_none() && ui.button("Open Project").clicked() {
                    self.open_file();
                    ui.close_menu();
                }

                if ui.button("Quit").clicked() {
                    frame.close();
                }
            });
        });
    }

    fn open_file(&mut self) {
        let main = dispatch::Queue::main();

        let (tx, rx) = channel();

        main.exec_async(move || {
            let path = FileDialog::new()
                .add_filter("csv", &["csv"])
                .set_directory("~/Desktop")
                .pick_file();

            tx.send(path).unwrap();
        });

        self.wait_for_file = Some(rx);
    }
}

impl eframe::App for App {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        if let Some(project) = &self.project {
            if let Err(e) = project.save() {
                tracing::error!("saving error: {}", e);
            }
        }
        eframe::set_value(storage, APP_KEY, self);
    }

    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:
            self.draw_menu(ui, frame);
        });

        if let Some(project) = self.project.as_mut() {
            egui::CentralPanel::default().show(ctx, |ui| {
                project.draw(ctx, ui);
            });
        }

        if let Some(receiver) = self.wait_for_file.as_ref() {
            if let Ok(received) = receiver.try_recv() {
                self.input_file = received;

                if let Some(input_file) = &self.input_file {
                    self.project = Project::new(input_file.clone()).ok();
                }

                self.wait_for_file = None;
            }
        }
    }
}
