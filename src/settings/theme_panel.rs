//! The Theme drawer region: a full-height sheet that slides in over the left
//! edge of the main window. An overlay rather than a floating `egui::Window`
//! (windows carry title-bar chrome that clashes with borderless mode) and
//! rather than a layout panel (pushing the fixed-column table aside smooshes
//! it at the small default window size). Overlaying costs the content nothing.
//! Owns its one piece of cross-frame state: whether it is open. Future ask
//! "new theme knob" lands here (plus the Theme field in config.rs, which is
//! domain and unavoidable).

use eframe::egui;

use super::chrome::{caption_button, CaptionIcon};
use super::style::{apply_style, rgb};
use super::widgets::toggle_switch;
use crate::config::Theme;

/// Outer sheet width, frame margins included. Sized to its own content only —
/// overlaying means the table's layout never depends on this number.
const WIDTH: f32 = 250.0;

pub struct ThemePanel {
    pub open: bool,
}

impl ThemePanel {
    pub fn new() -> Self {
        Self { open: false }
    }

    /// Renders (sliding in over the content) while open; applies the style
    /// live on any change; returns true when a theme field changed (the caller
    /// folds it into `committed`). An overlay in the foreground order — where
    /// it runs in the compose order doesn't affect any other region's layout.
    pub fn show(&mut self, ui: &mut egui::Ui, theme: &mut Theme) -> bool {
        let ctx = ui.ctx().clone();
        // slide: 0 = parked off-screen, 1 = fully out; skip everything once parked.
        // egui's default animation (0.12s linear) reads as a pop at this size —
        // a drawer wants a felt slide with an ease-out landing.
        let t = ctx.animate_bool_with_time_and_easing(
            egui::Id::new("theme-drawer-slide"),
            self.open,
            0.25,
            egui::emath::easing::cubic_out,
        );
        if t <= 0.0 {
            return false;
        }
        let mut changed = false;
        let mut close = false;
        let screen = ctx.content_rect();
        let frame = egui::Frame::new()
            .fill(ui.visuals().window_fill) // the derived raised-surface shade
            .stroke(ui.visuals().window_stroke) // themed hairline, both modes
            .shadow(egui::Shadow {
                offset: [3, 0],
                blur: 14,
                spread: 0,
                color: egui::Color32::from_black_alpha(45),
            })
            .inner_margin(egui::Margin::same(14));
        egui::Area::new(egui::Id::new("theme-drawer"))
            .order(egui::Order::Foreground)
            // THE load-bearing line: Areas default to constrain=true, which
            // clamps them inside the window every frame — silently deleting a
            // slide that parks off-screen. We own this position; no clamping.
            .constrain(false)
            // the sheet unmounts while parked, so every open is a "new" area to
            // egui: kill the new-area fade (it fights the slide) and declare the
            // size upfront (an unsized first frame paints short, then snaps)
            .fade_in(false)
            .default_size(egui::vec2(WIDTH, screen.height()))
            .fixed_pos(egui::pos2(screen.min.x - WIDTH * (1.0 - t), screen.min.y))
            .show(&ctx, |ui| {
                frame.show(ui, |ui| {
                    ui.set_width(WIDTH - 28.0); // frame margins account for the rest
                    ui.set_min_height(screen.height() - 28.0); // full-height sheet
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
                                    ui.ctx()
                                        .send_viewport_cmd(egui::ViewportCommand::Decorations(
                                            !theme.borderless,
                                        ));
                                    changed = true;
                                }
                                ui.end_row();
                            });
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
