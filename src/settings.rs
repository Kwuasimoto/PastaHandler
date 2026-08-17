use eframe::egui;

use crate::{
    config::{Config, ConfigFile, Theme},
    error::AppError,
};

mod style;

use style::{apply_style, install_fonts, mascot_for, on_color, rgb, scale};

pub fn run(config_file: ConfigFile) -> Result<(), AppError> {
    launch_gui(config_file)
}

struct SettingsApp {
    config_file: ConfigFile,
    draft: Config,
    error: Option<String>,
    /// Row index currently listening for a hotkey combo, if any.
    capturing: Option<usize>,
    /// Whether the Theme window is showing.
    theme_open: bool,
    /// Row whose delete button was hovered last frame (drives its red tint —
    /// the tint must be chosen before this frame's hover state exists).
    hovered_delete: Option<usize>,
    /// Debug builds only: F12 toggles egui's live style editor.
    #[cfg(debug_assertions)]
    style_editor: bool,
}

#[derive(Clone, Copy, PartialEq)]
enum CaptionIcon {
    Minimize,
    Maximize,
    Restore,
    Close,
}

/// A title-bar caption button: painter-drawn icon (no font-glyph gambling),
/// backplate on hover — the close button's backplate is the Windows-convention red.
fn caption_button(ui: &mut egui::Ui, icon: CaptionIcon, danger: bool) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(34.0, 24.0), egui::Sense::click());
    if ui.is_rect_visible(rect) {
        let hovered = response.hovered();
        if hovered {
            let fill = if danger {
                egui::Color32::from_rgb(232, 17, 35) // Windows close-red
            } else {
                ui.visuals().widgets.hovered.weak_bg_fill
            };
            ui.painter()
                .rect_filled(rect, ui.visuals().widgets.hovered.corner_radius, fill);
        }
        let color = if danger && hovered {
            egui::Color32::WHITE
        } else {
            ui.visuals().text_color()
        };
        let stroke = egui::Stroke::new(1.4, color);
        let c = rect.center();
        let r = 4.5; // icon half-size
        let p = ui.painter();
        match icon {
            CaptionIcon::Minimize => {
                p.line_segment([egui::pos2(c.x - r, c.y), egui::pos2(c.x + r, c.y)], stroke);
            }
            CaptionIcon::Maximize => {
                p.rect_stroke(
                    egui::Rect::from_center_size(c, egui::vec2(2.0 * r, 2.0 * r)),
                    egui::CornerRadius::same(2),
                    stroke,
                    egui::StrokeKind::Inside,
                );
            }
            CaptionIcon::Restore => {
                // two offset rounded squares — the classic "restore" pair
                let small = 2.0 * r - 2.0;
                p.rect_stroke(
                    egui::Rect::from_min_size(
                        egui::pos2(c.x - r + 2.0, c.y - r),
                        egui::vec2(small, small),
                    ),
                    egui::CornerRadius::same(2),
                    stroke,
                    egui::StrokeKind::Inside,
                );
                p.rect_stroke(
                    egui::Rect::from_min_size(
                        egui::pos2(c.x - r, c.y - r + 2.0),
                        egui::vec2(small, small),
                    ),
                    egui::CornerRadius::same(2),
                    stroke,
                    egui::StrokeKind::Inside,
                );
            }
            CaptionIcon::Close => {
                let d = r - 0.5;
                p.line_segment(
                    [egui::pos2(c.x - d, c.y - d), egui::pos2(c.x + d, c.y + d)],
                    stroke,
                );
                p.line_segment(
                    [egui::pos2(c.x - d, c.y + d), egui::pos2(c.x + d, c.y - d)],
                    stroke,
                );
            }
        }
    }
    response
}

/// Header theme button: a 2x2 grid of the CURRENT theme's colors — the control
/// previews its own subject. Ghost style with a hover backplate.
fn theme_swatch_button(ui: &mut egui::Ui, theme: &Theme) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(32.0, 26.0), egui::Sense::click());
    if ui.is_rect_visible(rect) {
        if response.hovered() {
            ui.painter().rect_filled(
                rect,
                ui.visuals().widgets.hovered.corner_radius,
                ui.visuals().widgets.hovered.weak_bg_fill,
            );
        }
        let s = 8.0;
        let gap = 2.0;
        let total = 2.0 * s + gap;
        let origin = rect.center() - egui::vec2(total / 2.0, total / 2.0);
        let colors = [
            rgb(theme.accent),
            rgb(theme.background),
            rgb(theme.text),
            rgb(theme.border),
        ];
        for (i, color) in colors.iter().enumerate() {
            let x = (i % 2) as f32 * (s + gap);
            let y = (i / 2) as f32 * (s + gap);
            let sw = egui::Rect::from_min_size(origin + egui::vec2(x, y), egui::vec2(s, s));
            ui.painter().rect_filled(sw, egui::CornerRadius::same(2), *color);
            ui.painter().rect_stroke(
                sw,
                egui::CornerRadius::same(2),
                egui::Stroke::new(1.0, rgb(theme.border)),
                egui::StrokeKind::Inside,
            );
        }
    }
    response.on_hover_text("Theme")
}

/// A small animated on/off switch (egui ships no built-in). Gold when on;
/// grayed and inert when `enabled` is false.
fn toggle_switch(
    ui: &mut egui::Ui,
    on: &mut bool,
    enabled: bool,
    accent: egui::Color32,
    knob_color: egui::Color32,
) -> egui::Response {
    // cell is wider than the track so it centers with breathing room, aligning
    // with the padded look of the other columns
    let cell = egui::vec2(40.0, 26.0);
    let (rect, mut response) = ui.allocate_exact_size(cell, egui::Sense::click());
    if response.clicked() && enabled {
        *on = !*on;
        response.mark_changed();
    }
    if ui.is_rect_visible(rect) {
        let track = egui::Rect::from_center_size(rect.center(), egui::vec2(34.0, 18.0));
        let t = ui.ctx().animate_bool(response.id, *on && enabled);
        let lerp8 = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t) as u8;
        let track_color = if enabled {
            let off = ui.visuals().widgets.active.weak_bg_fill; // themed "off" shade
            egui::Color32::from_rgb(
                lerp8(off.r(), accent.r()),
                lerp8(off.g(), accent.g()),
                lerp8(off.b(), accent.b()),
            )
        } else {
            ui.visuals().widgets.inactive.weak_bg_fill.linear_multiply(0.5)
        };
        // rectangular track, square sliding knob; radii follow the themed corner
        // radius so the shape slider drives the toggle too (0 = square, 12 = pill)
        let base = ui.visuals().widgets.inactive.corner_radius.nw;
        ui.painter()
            .rect_filled(track, egui::CornerRadius::same(base.min(9)), track_color);
        let knob = 12.0;
        let inset = 3.0;
        let knob_x = egui::lerp(
            (track.left() + inset + knob / 2.0)..=(track.right() - inset - knob / 2.0),
            t,
        );
        let knob_color = if enabled {
            knob_color
        } else {
            knob_color.linear_multiply(0.45)
        };
        ui.painter().rect_filled(
            egui::Rect::from_center_size(
                egui::pos2(knob_x, track.center().y),
                egui::vec2(knob, knob),
            ),
            egui::CornerRadius::same((base / 2).min(6)),
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

            // ONE deck: mascot left (+ small window title when chromeless), actions
            // right, caption controls rightmost. Whole deck drags in borderless mode.
            if self.draft.theme.borderless {
                let deck_rect = {
                    let mut r = ui.max_rect();
                    r.max.y = r.min.y + 56.0;
                    r
                };
                let ctx = ui.ctx().clone();
                let maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
                let bar = ui.interact(
                    deck_rect,
                    egui::Id::new("titlebar-deck"),
                    egui::Sense::click_and_drag(),
                );
                if bar.drag_started_by(egui::PointerButton::Primary) {
                    ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                }
                if bar.double_clicked() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
                }
            }

            // computed pre-theme-window: a background edit updates the mascot
            // next frame, by design
            let mascot = mascot_for(&self.draft.theme);

            ui.horizontal(|ui| {
                ui.add(
                    egui::Image::new(mascot.clone()).fit_to_exact_size(egui::vec2(63.0, 52.0)),
                );
                if self.draft.theme.borderless {
                    // stands in for the OS title bar text that decorations provide
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new("PastaHandler Settings").weak());
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if self.draft.theme.borderless {
                        let ctx = ui.ctx().clone();
                        let maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
                        if caption_button(ui, CaptionIcon::Close, true).clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                        let max_icon = if maximized {
                            CaptionIcon::Restore
                        } else {
                            CaptionIcon::Maximize
                        };
                        if caption_button(ui, max_icon, false).clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
                        }
                        if caption_button(ui, CaptionIcon::Minimize, false).clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                        }
                        ui.add_space(10.0);
                    }
                    // primary button: accent-filled — accent's permanent, visible home
                    let header_accent = rgb(self.draft.theme.accent);
                    let add = egui::Button::new(
                        egui::RichText::new("+  Add snippet").color(on_color(header_accent)),
                    )
                    .fill(header_accent);
                    if ui.add(add).clicked() {
                        self.draft.snippets.push(crate::config::Snippet {
                            text: String::new(),
                            hotkey: String::new(),
                            active: false, // capture (or the toggle) activates it later
                        });
                    }
                    if theme_swatch_button(ui, &self.draft.theme).clicked() {
                        self.theme_open = !self.theme_open;
                    }
                });
            });
            ui.add_space(8.0);
            // inset hairline — softer than a full-bleed rule
            let sep_rect = ui.max_rect();
            let sep_y = ui.cursor().top();
            ui.painter().hline(
                (sep_rect.left() + 8.0)..=(sep_rect.right() - 8.0),
                sep_y,
                ui.visuals().widgets.noninteractive.bg_stroke,
            );
            ui.add_space(14.0);

            if self.theme_open {
                let ctx = ui.ctx().clone();
                let mut open = self.theme_open;
                let mut changed = false;
                egui::Window::new("Theme")
                    .open(&mut open)
                    .resizable(false)
                    .collapsible(false)
                    .default_width(248.0)
                    .vscroll(true) // short main window => theme sections scroll into reach
                    .show(&ctx, |ui| {
                        ui.spacing_mut().interact_size = egui::vec2(56.0, 24.0); // wider swatches
                        ui.spacing_mut().slider_width = 130.0;

                        ui.label(egui::RichText::new("PRESETS").small().weak());
                        ui.add_space(2.0);
                        ui.horizontal(|ui| {
                            for (name, preset) in
                                [("Default", Theme::default()), ("Sakura", Theme::sakura())]
                            {
                                if ui.button(name).clicked() {
                                    // presets restyle; the window-border choice is yours to keep
                                    let borderless = self.draft.theme.borderless;
                                    self.draft.theme = preset;
                                    self.draft.theme.borderless = borderless;
                                    changed = true;
                                }
                            }
                        });

                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(6.0);

                        ui.label(egui::RichText::new("COLORS").small().weak());
                        ui.add_space(2.0);
                        egui::Grid::new("theme-colors")
                            .num_columns(2)
                            .min_col_width(104.0)
                            .spacing([12.0, 8.0])
                            .show(ui, |ui| {
                                for (label, color) in [
                                    ("Accent", &mut self.draft.theme.accent),
                                    ("Background", &mut self.draft.theme.background),
                                    ("Text", &mut self.draft.theme.text),
                                    ("Border", &mut self.draft.theme.border),
                                    ("Toggle knob", &mut self.draft.theme.knob),
                                ] {
                                    ui.label(label);
                                    changed |= ui.color_edit_button_srgb(color).changed();
                                    ui.end_row();
                                }
                            });

                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(6.0);

                        ui.label(egui::RichText::new("SHAPE").small().weak());
                        ui.add_space(2.0);
                        egui::Grid::new("theme-shape")
                            .num_columns(2)
                            .min_col_width(104.0)
                            .spacing([12.0, 8.0])
                            .show(ui, |ui| {
                                ui.label("Corner radius");
                                changed |= ui
                                    .add(egui::Slider::new(
                                        &mut self.draft.theme.corner_radius,
                                        0..=12,
                                    ))
                                    .changed();
                                ui.end_row();
                            });

                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(6.0);

                        ui.label(egui::RichText::new("WINDOW").small().weak());
                        ui.add_space(2.0);
                        egui::Grid::new("theme-window")
                            .num_columns(2)
                            .min_col_width(104.0)
                            .spacing([12.0, 8.0])
                            .show(ui, |ui| {
                                ui.label("Borderless");
                                let acc = rgb(self.draft.theme.accent);
                                let kn = rgb(self.draft.theme.knob);
                                let resp = toggle_switch(
                                    ui,
                                    &mut self.draft.theme.borderless,
                                    true,
                                    acc,
                                    kn,
                                );
                                if resp.changed() {
                                    // applies live — no restart needed
                                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Decorations(
                                        !self.draft.theme.borderless,
                                    ));
                                    changed = true;
                                }
                                ui.end_row();
                            });
                    });
                self.theme_open = open;
                if changed {
                    apply_style(&ctx, &self.draft.theme); // live restyle, same frame
                    committed = true; // themes auto-save like everything else
                }
            }

            let accent = rgb(self.draft.theme.accent);
            let knob = rgb(self.draft.theme.knob);
            let dim = scale(self.draft.theme.text, 0.52); // inactive-row text + ghost icons

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
                        let toggle =
                            toggle_switch(ui, &mut snippet.active, can_activate, accent, knob);
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
                            text_edit = text_edit.text_color(dim);
                        }
                        let r_text = ui.add_sized([text_w, 26.0], text_edit);
                        // Hotkey cell: a button that captures the next key combo when armed.
                        if self.capturing == Some(i) {
                            let resp = ui.add_sized(
                                [HOTKEY_W, 26.0],
                                egui::Button::new(
                                    egui::RichText::new("press keys…").italics().color(accent),
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
                                egui::RichText::new(&snippet.hotkey).color(dim)
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
                            dim
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

            // borderless windows lose the OS resize edges — provide the classic
            // corner grip in their place
            if self.draft.theme.borderless {
                let screen = ui.ctx().content_rect(); // egui 0.36: was screen_rect
                let grip = 20.0;
                let grip_rect =
                    egui::Rect::from_min_max(screen.max - egui::vec2(grip, grip), screen.max);
                let resp = ui
                    .interact(grip_rect, egui::Id::new("resize-grip-se"), egui::Sense::drag())
                    .on_hover_cursor(egui::CursorIcon::ResizeNwSe)
                    .on_hover_text("Drag to resize the window");
                if resp.drag_started_by(egui::PointerButton::Primary) {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::BeginResize(
                        egui::ResizeDirection::SouthEast,
                    ));
                }
                let grip_color = if resp.hovered() { accent } else { dim };
                for i in 0..3 {
                    let off = 8.0 + i as f32 * 4.0;
                    ui.painter().line_segment(
                        [
                            egui::pos2(screen.max.x - off, screen.max.y - 6.0),
                            egui::pos2(screen.max.x - 6.0, screen.max.y - off),
                        ],
                        egui::Stroke::new(1.5, grip_color),
                    );
                }
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
                theme_open: false,
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
