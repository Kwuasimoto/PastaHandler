use eframe::egui;

use crate::{
    config::{Config, ConfigFile},
    error::AppError,
};

mod chrome;
mod header;
mod style;
mod theme_window;
mod widgets;

use style::{apply_style, install_fonts, mascot_for, Palette};
use theme_window::ThemeWindow;
use widgets::toggle_switch;

pub fn run(config_file: ConfigFile) -> Result<(), AppError> {
    launch_gui(config_file)
}

struct SettingsApp {
    config_file: ConfigFile,
    draft: Config,
    error: Option<String>,
    /// Row index currently listening for a hotkey combo, if any.
    capturing: Option<usize>,
    theme_window: ThemeWindow,
    /// Row whose delete button was hovered last frame (drives its red tint —
    /// the tint must be chosen before this frame's hover state exists).
    hovered_delete: Option<usize>,
    /// Debug builds only: F12 toggles egui's live style editor.
    #[cfg(debug_assertions)]
    style_editor: bool,
}

/// Build a global-hotkey combo string from egui input. global-hotkey's parser
/// accepts egui's friendly key names ("2", "A", "F5") directly — no mapping
/// table needed; `.parse::<HotKey>()` is the validator.
fn combo_from(modifiers: &egui::Modifiers, key: egui::Key) -> String {
    let mut combo = String::new();
    if modifiers.ctrl {
        combo.push_str("ctrl+");
    }
    if modifiers.alt {
        combo.push_str("alt+");
    }
    if modifiers.shift {
        combo.push_str("shift+");
    }
    combo.push_str(key.name());
    combo
}

impl eframe::App for SettingsApp {
    // eframe 0.36: the trait method is `ui` (not the older `update`), and panels
    // are shown inside the provided `Ui`, not from a Context.
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Dev-mode "HMR": F12 opens egui's live style editor — tweak spacing,
        // colors, and radii in the running app, then transcribe keepers into
        // apply_style(). Compiled out of release builds entirely.
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
            let mut delete: Option<usize> = None;
            let mut committed = false;
            let mut just_activated: Option<usize> = None;
            let mut hovered_delete_now: Option<usize> = None;

            // computed pre-theme-window: a background edit updates the mascot
            // next frame, by design
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
                // flipped BEFORE the theme window shows — same-frame open
                self.theme_window.open = !self.theme_window.open;
            }

            // theme window renders BEFORE the palette is built, so edits recolor
            // the table the same frame; themes auto-save like everything else
            let ctx = ui.ctx().clone();
            committed |= self.theme_window.show(&ctx, &mut self.draft.theme);

            // computed post-theme-window so edits recolor the table the same frame
            let palette = Palette::from_theme(&self.draft.theme);

            // fixed columns + flexible Text column that absorbs the remaining width
            const SPACING: f32 = 10.0;
            const TOGGLE_W: f32 = 40.0;
            const HOTKEY_W: f32 = 130.0;
            const DELETE_W: f32 = 36.0;
            let text_w = (ui.available_width() - TOGGLE_W - HOTKEY_W - DELETE_W - 3.0 * SPACING)
                .max(210.0);

            if self.draft.snippets.is_empty() {
                // empty state: a prompt, not a bare table staring into the void
                ui.add_space(28.0);
                ui.vertical_centered(|ui| {
                    ui.add(egui::Image::new(mascot.clone()).fit_to_exact_size(egui::vec2(63.0, 52.0)));
                    ui.add_space(10.0);
                    ui.label(
                        egui::RichText::new("No snippets yet — press +  Add snippet to create one.")
                            .weak(),
                    );
                });
                ui.add_space(28.0);
            } else {
            // snippet list scrolls; header above and footer below stay pinned
            let reserve = if self.error.is_some() { 66.0 } else { 42.0 };
            egui::ScrollArea::vertical()
                .auto_shrink([false, true])
                .max_height((ui.available_height() - reserve).max(80.0))
                .show(ui, |ui| {
            egui::Grid::new("snippets")
                .striped(true)
                .num_columns(3)
                .spacing([SPACING, 8.0])
                .show(ui, |ui| {
                    ui.label("");
                    ui.strong("Text");
                    // centered over its fixed column, matching the buttons beneath
                    ui.add_sized(
                        [HOTKEY_W, 18.0],
                        egui::Label::new(egui::RichText::new("Hotkey").strong()),
                    );
                    ui.strong("");
                    ui.end_row();

                    for (i, snippet) in self.draft.snippets.iter_mut().enumerate() {
                        // an unbound snippet can never be active
                        let can_activate = !snippet.hotkey.trim().is_empty();
                        if !can_activate {
                            snippet.active = false;
                        }
                        let was_active = snippet.active;
                        let toggle = toggle_switch(
                            ui,
                            &mut snippet.active,
                            can_activate,
                            palette.accent,
                            palette.knob,
                        );
                        if !can_activate {
                            toggle.on_hover_text("Set a hotkey first");
                        } else if toggle.changed() {
                            committed = true;
                            if !was_active && snippet.active {
                                just_activated = Some(i);
                            }
                        }

                        // add_sized forces the cell allocation — plain desired_width
                        // gets clamped by the Grid's cell sizing.
                        let mut text_edit = egui::TextEdit::singleline(&mut snippet.text)
                            .hint_text("Text to paste")
                            .vertical_align(egui::Align::Center);
                        if !snippet.active {
                            text_edit = text_edit.text_color(palette.dim);
                        }
                        let r_text = ui.add_sized([text_w, 26.0], text_edit);
                        // Hotkey cell: a button that captures the next key combo when armed.
                        if self.capturing == Some(i) {
                            let resp = ui.add_sized(
                                [HOTKEY_W, 26.0],
                                egui::Button::new(
                                    egui::RichText::new("press keys…")
                                        .italics()
                                        .color(palette.accent),
                                ),
                            );
                            let events = ui.input(|inp| inp.events.clone());
                            for ev in events {
                                if let egui::Event::Key {
                                    key,
                                    physical_key,
                                    pressed: true,
                                    repeat: false,
                                    modifiers,
                                    ..
                                } = ev
                                {
                                    let key = physical_key.unwrap_or(key); // layout-independent
                                    if key == egui::Key::Escape {
                                        self.capturing = None;
                                        break;
                                    }
                                    // bare keys would hijack normal typing system-wide;
                                    // require at least one modifier
                                    if !(modifiers.ctrl || modifiers.alt || modifiers.shift) {
                                        continue;
                                    }
                                    let combo = combo_from(&modifiers, key);
                                    if combo.parse::<global_hotkey::hotkey::HotKey>().is_ok() {
                                        snippet.hotkey = combo;
                                        snippet.active = true; // capture implies intent to use
                                        self.capturing = None;
                                        committed = true;
                                        just_activated = Some(i);
                                        break;
                                    }
                                }
                            }
                            if resp.clicked() {
                                self.capturing = None; // clicking again cancels
                            }
                        } else {
                            let label = if snippet.hotkey.is_empty() {
                                egui::RichText::new("click to set").weak().italics()
                            } else if snippet.active {
                                egui::RichText::new(&snippet.hotkey)
                            } else {
                                egui::RichText::new(&snippet.hotkey).color(palette.dim)
                            };
                            if ui
                                .add_sized([HOTKEY_W, 26.0], egui::Button::new(label))
                                .on_hover_text("Click, then press the key combo (Esc cancels)")
                                .clicked()
                            {
                                self.capturing = Some(i);
                            }
                        }
                        // lost_focus fires on click-away AND on Enter — one signal covers both
                        committed |= r_text.lost_focus();
                        // ghost icon button, the production convention for row
                        // utilities: no frame at rest, muted icon; red when aimed at
                        let trash_tint = if self.hovered_delete == Some(i) {
                            egui::Color32::from_rgb(224, 82, 82)
                        } else {
                            palette.dim
                        };
                        // white source asset × tint = exact color
                        let trash = egui::Image::new(egui::include_image!("../assets/trash.svg"))
                            .fit_to_exact_size(egui::vec2(14.0, 14.0))
                            .tint(trash_tint);
                        let del_resp = ui
                            .add_sized([DELETE_W, 26.0], egui::Button::image(trash).frame(false))
                            .on_hover_text("Delete snippet");
                        if del_resp.hovered() {
                            hovered_delete_now = Some(i);
                        }
                        if del_resp.clicked() {
                            delete = Some(i);
                        }
                        ui.end_row();
                    }
                });

            }); // end scroll area
            } // end non-empty branch

            if let Some(i) = delete {
                self.draft.snippets.remove(i); // after the loop — the borrow is released here
                committed = true;
            }

            if committed {
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

            if let Some(err) = &self.error {
                ui.add_space(8.0);
                ui.colored_label(egui::Color32::from_rgb(220, 80, 80), err);
            }

            self.hovered_delete = hovered_delete_now;

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
                capturing: None,
                theme_window: ThemeWindow::new(),
                hovered_delete: None,
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
    use global_hotkey::hotkey::HotKey;

    fn mods(ctrl: bool, alt: bool, shift: bool) -> egui::Modifiers {
        egui::Modifiers { ctrl, alt, shift, ..Default::default() }
    }

    /// The property the whole capture feature rests on: combo strings built from
    /// egui key names are accepted by global-hotkey's parser.
    #[test]
    fn captured_combos_parse_as_hotkeys() {
        let cases = [
            (mods(true, true, false), egui::Key::Num2, "ctrl+alt+2"),
            (mods(true, false, false), egui::Key::A, "ctrl+A"),
            (mods(false, true, true), egui::Key::F5, "alt+shift+F5"),
            (mods(true, false, false), egui::Key::Space, "ctrl+Space"),
            (mods(true, false, true), egui::Key::Minus, "ctrl+shift+Minus"),
        ];
        for (m, key, expected) in cases {
            let combo = combo_from(&m, key);
            assert_eq!(combo, expected);
            combo
                .parse::<HotKey>()
                .unwrap_or_else(|e| panic!("'{combo}' must parse: {e:?}"));
        }
    }

    #[test]
    fn navigation_keys_parse_too() {
        for key in [egui::Key::ArrowDown, egui::Key::ArrowUp, egui::Key::Home, egui::Key::Tab] {
            let combo = combo_from(&mods(true, true, false), key);
            assert!(combo.parse::<HotKey>().is_ok(), "'{combo}' failed to parse");
        }
    }
}
