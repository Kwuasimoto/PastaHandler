use eframe::egui;

use crate::{
    config::{Config, ConfigFile},
    error::AppError,
};

mod chrome;
mod header;
mod snippet_table;
mod style;
mod theme_window;
mod widgets;

use snippet_table::SnippetTable;
use style::{apply_style, install_fonts, mascot_for, Palette};
use theme_window::ThemeWindow;

/// The settings process's single public door — main.rs and the resident depend
/// only on this signature.
pub fn run(config_file: ConfigFile) -> Result<(), AppError> {
    launch_gui(config_file)
}

struct SettingsApp {
    config_file: ConfigFile,
    draft: Config,
    error: Option<String>,
    theme_window: ThemeWindow,
    table: SnippetTable,
    /// Debug builds only: F12 toggles egui's live style editor.
    #[cfg(debug_assertions)]
    style_editor: bool,
}

impl SettingsApp {
    /// The save path: validate → write → activation-rollback on conflict.
    /// Sets `self.error` (None on success).
    fn commit(&mut self, just_activated: Option<usize>) {
        match crate::hotkeys::parse_all(&self.draft) {
            Ok(_) => {
                self.error = None;
                if let Err(e) = self.config_file.write(&self.draft) {
                    self.error = Some(e.to_string());
                }
            }
            Err(e) => {
                // if activating THIS commit caused the conflict, roll the
                // activation back so the draft stays saveable
                let rolled_back = just_activated.is_some_and(|ai| {
                    self.draft.snippets[ai].active = false;
                    crate::hotkeys::parse_all(&self.draft).is_ok()
                });
                if rolled_back {
                    if let Err(w) = self.config_file.write(&self.draft) {
                        self.error = Some(w.to_string());
                    } else {
                        self.error = Some(format!("{e} — snippet saved inactive"));
                    }
                } else {
                    // file untouched — its last good state stands; the
                    // resident never sees the invalid draft
                    self.error = Some(e.to_string());
                }
            }
        }
    }
}

impl eframe::App for SettingsApp {
    // eframe 0.36: the trait method is `ui` (not the older `update`), and panels
    // are shown inside the provided `Ui`, not from a Context.
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // COMPOSE ORDER — each numbered constraint is semantics, not style:
        //  1. F12 debug style editor (independent overlay)
        //  2. mode-aware margin + CentralPanel
        //  3. mascot picked PRE-theme-window (bg edits update it next frame, by design)
        //  4. header (its drag-interact registers before its buttons — hit-test order)
        //  5. header events applied (theme_open flips BEFORE the window shows)
        //  6. theme window
        //  7. palette built POST-theme-window (edits recolor the table the same frame)
        //  8. error_shown read PRE-commit (last frame's error sizes the scroll reserve)
        //  9. table → commit → error label, in that order
        // 10. footer, then the grip last so it paints over the corner

        // Dev-mode "HMR": F12 opens egui's live style editor — tweak values in
        // the running app, transcribe keepers into apply_style(). Debug only.
        #[cfg(debug_assertions)]
        {
            if ui.input(|i| i.key_pressed(egui::Key::F12)) {
                self.style_editor = !self.style_editor;
            }
            if self.style_editor {
                let ctx = ui.ctx().clone();
                egui::Window::new("Style editor (F12)")
                    .default_size([360.0, 520.0])
                    .show(&ctx, |ui| {
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            ctx.style_ui(ui, egui::Theme::Dark);
                        });
                    });
            }
        }

        // borderless: the title strip hugs the top like native caption buttons do;
        // decorated: the OS bar exists, so content keeps its comfortable margin
        let margin = if self.draft.theme.borderless {
            egui::Margin { left: 10, right: 10, top: 4, bottom: 10 }
        } else {
            egui::Margin { left: 24, right: 24, top: 16, bottom: 10 }
        };
        let panel_frame = egui::Frame::central_panel(ui.style()).inner_margin(margin);
        egui::CentralPanel::default().frame(panel_frame).show(ui, |ui| {
            let mascot = mascot_for(&self.draft.theme);

            let header_out = header::show(ui, &self.draft.theme, mascot.clone());
            if header_out.add_snippet {
                self.draft.snippets.push(crate::config::Snippet {
                    text: String::new(),
                    hotkey: String::new(),
                    active: false, // capture (or the toggle) activates it later
                });
            }
            if header_out.toggle_theme {
                self.theme_window.open = !self.theme_window.open;
            }

            let ctx = ui.ctx().clone();
            let mut committed = self.theme_window.show(&ctx, &mut self.draft.theme);

            let palette = Palette::from_theme(&self.draft.theme);
            let error_shown = self.error.is_some();
            let table_out =
                self.table
                    .show(ui, &mut self.draft.snippets, &palette, mascot, error_shown);
            committed |= table_out.committed;

            if committed {
                self.commit(table_out.just_activated);
            }

            if let Some(err) = &self.error {
                ui.add_space(8.0);
                ui.colored_label(egui::Color32::from_rgb(220, 80, 80), err);
            }

            // footer hint — pinned to the window's bottom edge, status-bar style
            ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                ui.label(
                    egui::RichText::new("Press a snippet's hotkey anywhere, then Ctrl+V to paste.")
                        .weak()
                        .small(),
                );
            });

            if self.draft.theme.borderless {
                chrome::resize_grip(ui, &palette);
            }
        });
    }
}

fn launch_gui(config_file: ConfigFile) -> Result<(), AppError> {
    let draft = config_file.read()?;
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([640.0, 400.0])
            .with_min_inner_size([560.0, 300.0])
            .with_max_inner_size([1000.0, 800.0])
            .with_decorations(!draft.theme.borderless),
        ..Default::default()
    };
    eframe::run_native(
        "PastaHandler Settings",
        options,
        Box::new(move |cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx); // enables SVG assets
            install_fonts(&cc.egui_ctx);
            apply_style(&cc.egui_ctx, &draft.theme);
            Ok(Box::new(SettingsApp {
                config_file,
                draft,
                error: None,
                theme_window: ThemeWindow::new(),
                table: SnippetTable::new(),
                #[cfg(debug_assertions)]
                style_editor: false,
            }))
        }),
    )
    .map_err(|e| AppError::Config(format!("settings window failed: {e}")))
}
