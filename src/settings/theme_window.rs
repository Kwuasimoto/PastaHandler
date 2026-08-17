//! The floating Theme window region. Owns its one piece of cross-frame state:
//! whether it is open. Future ask "new theme knob" lands here (plus the Theme
//! field in config.rs, which is domain and unavoidable).

use eframe::egui;

use super::style::{apply_style, rgb};
use super::widgets::toggle_switch;
use crate::config::Theme;

pub struct ThemeWindow {
    pub open: bool,
}

impl ThemeWindow {
    pub fn new() -> Self {
        Self { open: false }
    }

    /// Renders when open; applies the style live on any change; returns true
    /// when a theme field changed (the caller folds it into `committed`).
    ///
    /// Borrow note: `.open(&mut self.open)` compiles because the window closure
    /// captures only the `theme` PARAMETER, never `self`. If this struct ever
    /// grows a second field the closure needs, the local-copy dance returns.
    pub fn show(&mut self, ctx: &egui::Context, theme: &mut Theme) -> bool {
        if !self.open {
            return false;
        }
        let mut changed = false;
        egui::Window::new("Theme")
            .open(&mut self.open)
            .resizable(false)
            .collapsible(false)
            .default_width(248.0)
            .vscroll(true) // short main window => theme sections scroll into reach
            .show(ctx, |ui| {
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
                    .min_col_width(104.0)
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
                    .min_col_width(104.0)
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
                    .min_col_width(104.0)
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
        if changed {
            apply_style(ctx, theme); // live restyle, same frame
        }
        changed
    }
}
