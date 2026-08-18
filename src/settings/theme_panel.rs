//! The Theme drawer region: a left panel that slides out of the main window.
//! A panel rather than a floating `egui::Window` because windows carry their
//! own title-bar chrome — wrong twice over when the user has turned window
//! borders OFF. A panel is identical in both border modes. Owns its one piece
//! of cross-frame state: whether it is open. Future ask "new theme knob" lands
//! here (plus the Theme field in config.rs, which is domain and unavoidable).

use eframe::egui;

use super::chrome::{caption_button, CaptionIcon};
use super::style::{apply_style, rgb};
use super::widgets::toggle_switch;
use crate::config::Theme;

/// Outer drawer width, frame margins included. Sized against the snippet
/// table: at the default 660px window, full rows still fit beside the open
/// drawer in both border modes.
const WIDTH: f32 = 250.0;

pub struct ThemePanel {
    pub open: bool,
}

impl ThemePanel {
    pub fn new() -> Self {
        Self { open: false }
    }

    /// Renders (with egui's slide animation) while open; applies the style
    /// live on any change; returns true when a theme field changed (the caller
    /// folds it into `committed`). Must run BEFORE the central panel — side
    /// panels claim their width first.
    pub fn show(&mut self, ui: &mut egui::Ui, theme: &mut Theme) -> bool {
        let mut changed = false;
        let mut close = false;
        let frame = egui::Frame::new()
            .fill(ui.visuals().window_fill) // the derived raised-surface shade
            .inner_margin(egui::Margin::same(14));
        egui::Panel::left("theme-drawer")
            .resizable(false)
            .drag_to_open(false) // no accidental edge-drag opens; the swatch button is the door
            .default_size(WIDTH)
            .frame(frame)
            .show_collapsible(ui, &mut self.open, |ui| {
                ui.spacing_mut().interact_size = egui::vec2(56.0, 24.0); // wider swatches
                ui.spacing_mut().slider_width = 64.0; // + its value box stays inside WIDTH

                // drawer header: title left, quiet ghost close right
                ui.horizontal(|ui| {
                    ui.strong("Theme");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if caption_button(ui, CaptionIcon::Close, false).clicked() {
                            close = true;
                        }
                    });
                });
                ui.add_space(6.0);

                // short main window => theme sections scroll into reach
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.label(egui::RichText::new("PRESETS").small().weak());
                    ui.add_space(2.0);
                    ui.horizontal(|ui| {
                        for (name, preset) in
                            [("Default", Theme::default()), ("Sakura", Theme::sakura())]
                        {
                            if ui.button(name).clicked() {
                                // presets restyle; the window-border choice is yours to keep
                                let borderless = theme.borderless;
                                *theme = preset;
                                theme.borderless = borderless;
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
                        .min_col_width(96.0)
                        .spacing([12.0, 8.0])
                        .show(ui, |ui| {
                            for (label, color) in [
                                ("Accent", &mut theme.accent),
                                ("Background", &mut theme.background),
                                ("Text", &mut theme.text),
                                ("Border", &mut theme.border),
                                ("Toggle knob", &mut theme.knob),
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
                        .min_col_width(96.0)
                        .spacing([12.0, 8.0])
                        .show(ui, |ui| {
                            ui.label("Corner radius");
                            changed |= ui
                                .add(egui::Slider::new(&mut theme.corner_radius, 0..=12))
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
                        .min_col_width(96.0)
                        .spacing([12.0, 8.0])
                        .show(ui, |ui| {
                            ui.label("Borderless");
                            let acc = rgb(theme.accent);
                            let kn = rgb(theme.knob);
                            let resp = toggle_switch(ui, &mut theme.borderless, true, acc, kn);
                            if resp.changed() {
                                // applies live — no restart needed
                                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Decorations(
                                    !theme.borderless,
                                ));
                                changed = true;
                            }
                            ui.end_row();
                        });
                });
            });
        if close {
            self.open = false;
        }
        if changed {
            apply_style(ui.ctx(), theme); // live restyle, same frame
        }
        changed
    }
}
