use eframe::egui;

use crate::{
    config::{Config, ConfigFile},
    error::AppError,
};

pub fn run(config_file: ConfigFile) -> Result<(), AppError> {
    launch_gui(config_file)
}

struct SettingsApp {
    config_file: ConfigFile,
    draft: Config,
    error: Option<String>,
}

impl eframe::App for SettingsApp {
    // eframe 0.36: the trait method is `ui` (not the older `update`), and panels
    // are shown inside the provided `Ui`, not from a Context.
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |ui| {
            ui.add_space(6.0);
            ui.heading("PastaHandler — Snippets");
            ui.label(
                egui::RichText::new(
                    "Assign a hotkey to each snippet — press it in-game, then Ctrl+V to paste.",
                )
                .weak(),
            );
            ui.add_space(14.0);

            let mut delete: Option<usize> = None;
            let mut committed = false;

            egui::Grid::new("snippets")
                .striped(true)
                .num_columns(4)
                .spacing([10.0, 8.0])
                .show(ui, |ui| {
                    ui.strong("Label");
                    ui.strong("Text");
                    ui.strong("Hotkey");
                    ui.strong("");
                    ui.end_row();

                    for (i, snippet) in self.draft.snippets.iter_mut().enumerate() {
                        // add_sized forces the cell allocation — plain desired_width
                        // gets clamped by the Grid's cell sizing.
                        let r1 = ui.add_sized(
                            [110.0, 22.0],
                            egui::TextEdit::singleline(&mut snippet.label).hint_text("Label"),
                        );
                        let r2 = ui.add_sized(
                            [210.0, 22.0],
                            egui::TextEdit::singleline(&mut snippet.text).hint_text("Text to paste"),
                        );
                        let r3 = ui.add_sized(
                            [120.0, 22.0],
                            egui::TextEdit::singleline(&mut snippet.hotkey)
                                .hint_text("ctrl+alt+Digit1"),
                        );
                        // lost_focus fires on click-away AND on Enter — one signal covers both
                        committed |= r1.lost_focus() || r2.lost_focus() || r3.lost_focus();
                        if ui.button("Delete").clicked() {
                            delete = Some(i);
                        }
                        ui.end_row();
                    }
                });

            if let Some(i) = delete {
                self.draft.snippets.remove(i); // after the loop — the borrow is released here
                committed = true;
            }

            ui.add_space(12.0);
            // Add deliberately does NOT commit: a fresh row has an empty hotkey and
            // would instantly red-flag; it writes when filled in and clicked away.
            if ui.button("+  Add snippet").clicked() {
                self.draft.snippets.push(crate::config::Snippet {
                    label: "New".into(),
                    text: String::new(),
                    hotkey: String::new(),
                });
            }

            if committed {
                match crate::hotkeys::parse_all(&self.draft) {
                    Ok(_) => {
                        self.error = None;
                        if let Err(e) = self.config_file.write(&self.draft) {
                            self.error = Some(e.to_string());
                        }
                    }
                    // file untouched — its last good state stands; the resident never
                    // sees the invalid draft
                    Err(e) => self.error = Some(e.to_string()),
                }
            }

            if let Some(err) = &self.error {
                ui.add_space(8.0);
                ui.colored_label(egui::Color32::from_rgb(220, 80, 80), err);
            }
        });
    }
}

fn launch_gui(config_file: ConfigFile) -> Result<(), AppError> {
    let draft = config_file.read()?;
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([560.0, 400.0])
            .with_min_inner_size([560.0, 300.0]),
        ..Default::default()
    };
    eframe::run_native(
        "PastaHandler Settings",
        options,
        Box::new(move |_cc| Ok(Box::new(SettingsApp { config_file, draft, error: None }))),
    )
    .map_err(|e| AppError::Config(format!("settings window failed: {e}")))
}
