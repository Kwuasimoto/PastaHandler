//! The settings process: composition shell for the egui UI regions (see the
//! COMPOSE ORDER comment in `ui()`), the domain state, and the save path.

use eframe::egui;

use crate::{
    config::{Config, ConfigFile},
    error::AppError,
};

mod chrome;
mod header;
mod snippet_table;
mod style;
mod theme_panel;
mod widgets;

use snippet_table::SnippetTable;
use style::{
    apply_style, install_example_bg, install_fonts, mascot_for, mascot_smile_for,
    paint_background, Palette,
};
use theme_panel::ThemePanel;

/// The settings process's single public door — main.rs and the resident depend
/// only on this signature.
pub fn run(config_file: ConfigFile) -> Result<(), AppError> {
    // one settings window is the app's contract: a second launch (tray click,
    // Start menu, double-tap) focuses the existing window instead of twinning.
    // Debug capture rigs (PASTAHANDLER_DRAWER) spawn extra instances freely.
    let test_rig = cfg!(debug_assertions) && std::env::var_os("PASTAHANDLER_DRAWER").is_some();
    if !test_rig && !crate::win32::claim_single_instance("Local\\pastahandler-settings") {
        crate::win32::focus_settings_window();
        return Ok(());
    }
    launch_gui(config_file)
}

struct SettingsApp {
    config_file: ConfigFile,
    draft: Config,
    error: Option<String>,
    theme_panel: ThemePanel,
    table: SnippetTable,
    /// Last focus_outline value pushed to DWM — apply-on-change, not per-frame.
    applied_outline: Option<bool>,
    /// Last blur value pushed to the DWM accent policy, same pattern.
    applied_blur: Option<bool>,
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
    // fully transparent surface every frame: paint_background owns the canvas,
    // and the DWM accent policy (win32::set_blur) composites the desktop
    // behind whatever alpha the canvas leaves open
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }

    // eframe 0.36: the trait method is `ui` (not the older `update`), and panels
    // are shown inside the provided `Ui`, not from a Context.
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        // COMPOSE ORDER — each numbered constraint is semantics, not style:
        //  1. F12 debug style editor (independent overlay)
        //  2. theme drawer — an overlay (Foreground order), so it costs the
        //     content no layout. Shown before the header, so its toggle lands
        //     next frame; the slide animation makes that lag invisible.
        //  3. mode-aware margin + CentralPanel
        //  4. mascot picked POST-drawer (bg edits restyle it the same frame)
        //  5. header (its drag-interact registers before its buttons — hit-test order)
        //  6. header events applied (drawer toggle — see 2)
        //  7. palette built POST-drawer (edits recolor the table the same frame)
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

        // the canvas first: theme color and optional image at the chosen
        // opacity — everything after paints (solid) on top of it
        paint_background(ui, &self.draft.theme);

        // window-level Win32 state follows the theme; applied on change only.
        // Our own HWND comes from eframe — never window enumeration in-frame
        // (same-process GetWindowTextW re-enters the wndproc and deadlocks).
        let outline_dirty = self.applied_outline != Some(self.draft.theme.focus_outline);
        let blur_dirty = self.applied_blur != Some(self.draft.theme.blur);
        if (outline_dirty || blur_dirty)
            && let Ok(handle) = raw_window_handle::HasWindowHandle::window_handle(frame)
            && let raw_window_handle::RawWindowHandle::Win32(w) = handle.as_raw()
        {
            if blur_dirty {
                crate::win32::set_blur(w.hwnd.get(), self.draft.theme.blur);
                self.applied_blur = Some(self.draft.theme.blur);
            }
            if outline_dirty {
                crate::win32::set_system_border(w.hwnd.get(), self.draft.theme.focus_outline);
                self.applied_outline = Some(self.draft.theme.focus_outline);
            }
        }

        let theme_committed = self.theme_panel.show(ui, &mut self.draft.theme);

        // borderless: the title strip hugs the top like native caption buttons do;
        // decorated: the OS bar exists, so content keeps its comfortable margin
        let margin = if self.draft.theme.borderless {
            egui::Margin { left: 10, right: 10, top: 4, bottom: 10 }
        } else {
            egui::Margin { left: 24, right: 24, top: 16, bottom: 10 }
        };
        // transparent fill: paint_background owns the canvas
        let panel_frame = egui::Frame::central_panel(ui.style())
            .fill(egui::Color32::TRANSPARENT)
            .inner_margin(margin);
        egui::CentralPanel::default().frame(panel_frame).show(ui, |ui| {
            let mascot = mascot_for(&self.draft.theme);

            let header_out = header::show(
                ui,
                &self.draft.theme,
                mascot.clone(),
                mascot_smile_for(&self.draft.theme),
            );
            if header_out.add_snippet {
                self.draft.snippets.push(crate::config::Snippet {
                    text: String::new(),
                    hotkey: String::new(),
                    active: false, // capture (or the toggle) activates it later
                });
            }
            if header_out.toggle_theme {
                self.theme_panel.open = !self.theme_panel.open;
            }

            let mut committed = theme_committed;

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
                ui.colored_label(palette.danger, err);
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
        // glow, not the default wgpu: wgpu presents with alpha-mode IGNORE on
        // Windows, so the canvas per-pixel alpha never reaches the DWM accent
        // layer. glow WGL presents carry the framebuffer alpha through.
        renderer: eframe::Renderer::Glow,
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([640.0, 400.0])
            .with_min_inner_size([560.0, 300.0])
            .with_max_inner_size([1000.0, 800.0])
            .with_decorations(!draft.theme.borderless)
            .with_transparent(true), // the canvas's per-pixel alpha needs an alpha surface
        ..Default::default()
    };
    eframe::run_native(
        crate::win32::SETTINGS_WINDOW_TITLE,
        options,
        Box::new(move |cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx); // enables SVG assets
            install_example_bg(&cc.egui_ctx); // embedded example background
            install_fonts(&cc.egui_ctx);
            apply_style(&cc.egui_ctx, &draft.theme);
            #[allow(unused_mut)]
            let mut theme_panel = ThemePanel::new();
            // dev-only test hook: capture rigs verify drawer layout hands-free
            #[cfg(debug_assertions)]
            if std::env::var_os("PASTAHANDLER_DRAWER").is_some() {
                theme_panel.open = true;
            }
            Ok(Box::new(SettingsApp {
                config_file,
                draft,
                error: None,
                theme_panel,
                table: SnippetTable::new(),
                applied_outline: None,
                applied_blur: None,
                #[cfg(debug_assertions)]
                style_editor: false,
            }))
        }),
    )
    .map_err(|e| AppError::Config(format!("settings window failed: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Snippet, Theme};

    fn snippet(text: &str, hotkey: &str, active: bool) -> Snippet {
        Snippet { text: text.into(), hotkey: hotkey.into(), active }
    }

    fn app_with(file_name: &str, snippets: Vec<Snippet>) -> SettingsApp {
        SettingsApp {
            config_file: ConfigFile::new(std::env::temp_dir().join(file_name)),
            draft: Config { snippets, open_settings_on_launch: false, theme: Theme::default() },
            error: None,
            theme_panel: ThemePanel::new(),
            table: SnippetTable::new(),
            applied_outline: None,
            applied_blur: None,
            #[cfg(debug_assertions)]
            style_editor: false,
        }
    }

    #[test]
    fn commit_valid_draft_writes_and_clears_error() {
        let mut app = app_with(
            "pastahandler-test-commit-ok.toml",
            vec![
                snippet("one", "ctrl+alt+Digit1", true),
                snippet("two", "ctrl+alt+Digit2", true),
            ],
        );
        app.error = Some("stale error from last time".into());
        app.commit(None);
        assert_eq!(app.error, None);
        let on_disk = app.config_file.read().expect("file was written");
        assert_eq!(on_disk, app.draft);
        let _ = std::fs::remove_file(app.config_file.path());
    }

    #[test]
    fn commit_conflict_with_activation_rolls_back_and_still_saves() {
        // two ACTIVE snippets sharing a combo; row 1 was just activated
        let mut app = app_with(
            "pastahandler-test-commit-rollback.toml",
            vec![
                snippet("first", "ctrl+alt+Digit1", true),
                snippet("twin", "ctrl+alt+Digit1", true),
            ],
        );
        app.commit(Some(1));
        // the activation was rolled back...
        assert!(!app.draft.snippets[1].active, "conflicting activation must roll back");
        // ...the rest still saved...
        let on_disk = app.config_file.read().expect("rollback still writes");
        assert_eq!(on_disk, app.draft);
        // ...and the user is told what happened
        let err = app.error.expect("explains the rollback");
        assert!(err.contains("saved inactive"), "message explains outcome: {err}");
        let _ = std::fs::remove_file(app.config_file.path());
    }

    #[test]
    fn commit_conflict_without_activation_leaves_file_untouched() {
        let mut app = app_with(
            "pastahandler-test-commit-blocked.toml",
            vec![snippet("good", "ctrl+alt+Digit1", true)],
        );
        app.commit(None); // establish a known-good file
        assert_eq!(app.error, None);

        // corrupt the draft with a duplicate that no activation explains
        app.draft.snippets.push(snippet("twin", "ctrl+alt+Digit1", true));
        app.commit(None);
        assert!(app.error.is_some(), "conflict must surface");
        let on_disk = app.config_file.read().expect("read back");
        assert_eq!(on_disk.snippets.len(), 1, "last good state stands on disk");
        let _ = std::fs::remove_file(app.config_file.path());
    }
}
