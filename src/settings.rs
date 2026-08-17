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
    /// Row index currently listening for a hotkey combo, if any.
    capturing: Option<usize>,
    /// Debug builds only: F12 toggles egui's live style editor.
    #[cfg(debug_assertions)]
    style_editor: bool,
}

/// Brand accent — the farfalle's pasta gold.
const GOLD: egui::Color32 = egui::Color32::from_rgb(242, 183, 53);

/// One-time widget styling: consistent radii and heights, roomier padding,
/// subtle borders, gold focus/hover accents.
fn apply_style(ctx: &egui::Context) {
    // egui 0.36 keeps a style per theme; mutate both so a future light theme inherits the look
    ctx.all_styles_mut(|style| {
        style.spacing.button_padding = egui::vec2(12.0, 5.0);
        style.spacing.item_spacing = egui::vec2(10.0, 8.0);
        style.spacing.interact_size.y = 26.0;

        let v = &mut style.visuals;
        v.text_edit_bg_color = Some(egui::Color32::from_gray(16));
        v.selection.bg_fill = GOLD.linear_multiply(0.25); // text selection wash
        v.selection.stroke = egui::Stroke::new(1.0, GOLD); // focused-input ring

        for w in [
            &mut v.widgets.noninteractive,
            &mut v.widgets.inactive,
            &mut v.widgets.hovered,
            &mut v.widgets.active,
            &mut v.widgets.open,
        ] {
            w.corner_radius = egui::CornerRadius::same(6);
        }
        v.widgets.inactive.weak_bg_fill = egui::Color32::from_gray(38);
        v.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_gray(58));
        v.widgets.hovered.weak_bg_fill = egui::Color32::from_gray(48);
        v.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, GOLD.linear_multiply(0.6));
        v.widgets.active.weak_bg_fill = egui::Color32::from_gray(55);
        v.widgets.active.bg_stroke = egui::Stroke::new(1.0, GOLD);
    });
}

/// A small animated on/off switch (egui ships no built-in). Gold when on;
/// grayed and inert when `enabled` is false.
fn toggle_switch(ui: &mut egui::Ui, on: &mut bool, enabled: bool) -> egui::Response {
    let cell = egui::vec2(34.0, 26.0); // matches the row height; track centers inside
    let (rect, mut response) = ui.allocate_exact_size(cell, egui::Sense::click());
    if response.clicked() && enabled {
        *on = !*on;
        response.mark_changed();
    }
    if ui.is_rect_visible(rect) {
        let track = egui::Rect::from_center_size(rect.center(), egui::vec2(34.0, 18.0));
        let radius = track.height() / 2.0;
        let t = ui.ctx().animate_bool(response.id, *on && enabled);
        let lerp8 = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t) as u8;
        let track_color = if enabled {
            let off = egui::Color32::from_gray(55);
            egui::Color32::from_rgb(
                lerp8(off.r(), GOLD.r()),
                lerp8(off.g(), GOLD.g()),
                lerp8(off.b(), GOLD.b()),
            )
        } else {
            egui::Color32::from_gray(32)
        };
        ui.painter()
            .rect_filled(track, egui::CornerRadius::same(radius as u8), track_color);
        let knob_x = egui::lerp((track.left() + radius)..=(track.right() - radius), t);
        let knob_color = if enabled {
            egui::Color32::from_gray(235)
        } else {
            egui::Color32::from_gray(85)
        };
        ui.painter().circle_filled(
            egui::pos2(knob_x, track.center().y),
            radius - 3.0,
            knob_color,
        );
    }
    response
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

        let panel_frame =
            egui::Frame::central_panel(ui.style()).inner_margin(egui::Margin::symmetric(24, 16));
        egui::CentralPanel::default().frame(panel_frame).show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.heading("PastaHandler — Snippets");
                ui.label(
                    egui::RichText::new(
                        "Assign a hotkey to each snippet — press it, then Ctrl+V to paste.",
                    )
                    .weak(),
                );
            });
            ui.add_space(14.0);

            // fixed columns + flexible Text column that absorbs the remaining width
            const SPACING: f32 = 10.0;
            const TOGGLE_W: f32 = 34.0;
            const HOTKEY_W: f32 = 130.0;
            const DELETE_W: f32 = 36.0;
            let text_w = (ui.available_width() - TOGGLE_W - HOTKEY_W - DELETE_W - 3.0 * SPACING)
                .max(210.0);

            let mut delete: Option<usize> = None;
            let mut committed = false;
            let mut just_activated: Option<usize> = None;

            egui::Grid::new("snippets")
                .striped(true)
                .num_columns(3)
                .spacing([SPACING, 8.0])
                .show(ui, |ui| {
                    ui.label("");
                    ui.strong("Text");
                    ui.strong("Hotkey");
                    ui.strong("");
                    ui.end_row();

                    for (i, snippet) in self.draft.snippets.iter_mut().enumerate() {
                        // an unbound snippet can never be active
                        let can_activate = !snippet.hotkey.trim().is_empty();
                        if !can_activate {
                            snippet.active = false;
                        }
                        let was_active = snippet.active;
                        let toggle = toggle_switch(ui, &mut snippet.active, can_activate);
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
                            text_edit = text_edit.text_color(egui::Color32::from_gray(115));
                        }
                        let r_text = ui.add_sized([text_w, 26.0], text_edit);
                        // Hotkey cell: a button that captures the next key combo when armed.
                        if self.capturing == Some(i) {
                            let resp = ui.add_sized(
                                [HOTKEY_W, 26.0],
                                egui::Button::new(
                                    egui::RichText::new("press keys…").italics().color(GOLD),
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
                                egui::RichText::new(&snippet.hotkey)
                                    .color(egui::Color32::from_gray(115))
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
                        let trash = egui::Image::new(egui::include_image!("../assets/trash.svg"))
                            .fit_to_exact_size(egui::vec2(14.0, 14.0));
                        if ui
                            .add_sized([DELETE_W, 26.0], egui::Button::image(trash))
                            .on_hover_text("Delete snippet")
                            .clicked()
                        {
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
                    text: String::new(),
                    hotkey: String::new(),
                    active: false, // capture (or the toggle) activates it later
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
        });
    }
}

fn launch_gui(config_file: ConfigFile) -> Result<(), AppError> {
    let draft = config_file.read()?;
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([640.0, 400.0])
            .with_min_inner_size([560.0, 300.0])
            .with_max_inner_size([1000.0, 800.0]),
        ..Default::default()
    };
    eframe::run_native(
        "PastaHandler Settings",
        options,
        Box::new(move |cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx); // enables SVG assets
            apply_style(&cc.egui_ctx);
            Ok(Box::new(SettingsApp {
                config_file,
                draft,
                error: None,
                capturing: None,
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
