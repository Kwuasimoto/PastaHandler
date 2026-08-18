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

/// A preset as its own preview: card background/border/text/radius all come
/// from the PRESET (not the active theme), with an accent chip carrying a
/// knob dot. The active palette gets an accent selection ring.
fn preset_card(
    ui: &mut egui::Ui,
    name: &str,
    preset: &Theme,
    current: &Theme,
    width: f32,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, 40.0), egui::Sense::click());
    if ui.is_rect_visible(rect) {
        let selected = current.accent == preset.accent
            && current.background == preset.background
            && current.text == preset.text
            && current.border == preset.border
            && current.knob == preset.knob
            && current.corner_radius == preset.corner_radius;
        let radius = egui::CornerRadius::same(preset.corner_radius.min(12));
        ui.painter().rect_filled(rect, radius, rgb(preset.background));
        let stroke = if selected {
            egui::Stroke::new(2.0, rgb(preset.accent))
        } else if response.hovered() {
            egui::Stroke::new(1.5, rgb(preset.accent).linear_multiply(0.75))
        } else {
            egui::Stroke::new(1.0, rgb(preset.border))
        };
        ui.painter()
            .rect_stroke(rect, radius, stroke, egui::StrokeKind::Inside);
        // accent chip with the knob as a dot riding on it
        let chip = egui::Rect::from_center_size(
            egui::pos2(rect.left() + 17.0, rect.center().y),
            egui::vec2(14.0, 14.0),
        );
        ui.painter()
            .rect_filled(chip, egui::CornerRadius::same(4), rgb(preset.accent));
        ui.painter().circle_filled(
            egui::pos2(chip.right() - 3.5, chip.bottom() - 3.5),
            2.5,
            rgb(preset.knob),
        );
        ui.painter().text(
            egui::pos2(rect.left() + 30.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            name,
            egui::FontId::proportional(13.0),
            rgb(preset.text),
        );
    }
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

pub struct ThemePanel {
    pub open: bool,
    /// Preset gallery expansion — collapsed shows only the first row.
    presets_expanded: bool,
    /// In-flight native file dialog. rfd's pick_file BLOCKS its thread — run
    /// on the UI thread it freezes the window white if the dialog opens
    /// behind it. So it runs on its own thread and we poll the result here.
    picker: Option<std::sync::mpsc::Receiver<Option<std::path::PathBuf>>>,
}

impl ThemePanel {
    pub fn new() -> Self {
        Self { open: false, presets_expanded: false, picker: None }
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
        let mut presets_expanded = self.presets_expanded;
        let self_picker_active = self.picker.is_some();
        let mut picker_started: Option<std::sync::mpsc::Receiver<Option<std::path::PathBuf>>> =
            None;
        let screen = ctx.content_rect();
        // no frame stroke: a four-sided border collides with the window edge
        // and the OS focus ring at the flush top/left/bottom — the sheet's only
        // border is the right-edge hairline painted after the content
        let frame = egui::Frame::new()
            .fill(ui.visuals().window_fill) // the derived raised-surface shade
            .shadow(egui::Shadow {
                offset: [3, 0],
                blur: 14,
                spread: 0,
                color: egui::Color32::from_black_alpha(45),
            })
            // asymmetric on purpose: the right margin is 2 so the scrollbar and
            // the close button hug the sheet edge; the content's breathing room
            // from the bar is its own 8px inset inside the scroll area
            .inner_margin(egui::Margin { left: 14, right: 2, top: 14, bottom: 14 });
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
                    ui.set_width(WIDTH - 16.0); // frame margins account for the rest
                    ui.set_min_height(screen.height() - 28.0); // full-height sheet
                    ui.spacing_mut().interact_size = egui::vec2(56.0, 24.0); // wider swatches

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

                    // short main window => theme sections scroll into reach.
                    // auto_shrink OFF horizontally: the scroll lane fills the
                    // sheet so the bar pins to the edge — otherwise it shrinks
                    // to hug the content and rides along with any padding.
                    egui::ScrollArea::vertical().auto_shrink([false, true]).show(ui, |ui| {
                        // 12px: 8 of breathing room + 4 so the floating bar can
                        // fatten on hover without overlapping the content
                        ui.set_width(WIDTH - 16.0 - 12.0);
                        ui.label(egui::RichText::new("PRESETS").small().weak());
                        ui.add_space(2.0);
                        // two-column grid of live preview cards — each card IS
                        // its theme in miniature (bg, border, accent chip, text
                        // color, corner radius), so you read it before you click.
                        // Collapsed, only the first row shows — real estate.
                        let presets = Theme::presets();
                        let shown = if presets_expanded { presets.len() } else { 2 };
                        let card_w = (ui.available_width() - 8.0) / 2.0;
                        for pair in presets[..shown].chunks(2) {
                            ui.horizontal(|ui| {
                                for (name, preset) in pair {
                                    if preset_card(ui, name, preset, theme, card_w).clicked()
                                    {
                                        // presets are PALETTES: colors + radius
                                        // only; window behavior stays yours
                                        theme.accent = preset.accent;
                                        theme.background = preset.background;
                                        theme.text = preset.text;
                                        theme.border = preset.border;
                                        theme.knob = preset.knob;
                                        theme.corner_radius = preset.corner_radius;
                                        changed = true;
                                    }
                                }
                            });
                            ui.add_space(6.0);
                        }
                        let expander = if presets_expanded {
                            "Show less".to_owned()
                        } else {
                            format!("Show all ({} more)", presets.len() - shown)
                        };
                        if ui.link(egui::RichText::new(expander).small().weak()).clicked() {
                            presets_expanded = !presets_expanded;
                        }

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
                        // label + value up top, full-width slider underneath —
                        // the inline three-piece row didn't fit the sheet
                        ui.horizontal(|ui| {
                            ui.label("Corner radius");
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    changed |= ui
                                        .add(
                                            egui::DragValue::new(&mut theme.corner_radius)
                                                .range(0..=12),
                                        )
                                        .changed();
                                },
                            );
                        });
                        ui.scope(|ui| {
                            ui.spacing_mut().slider_width = ui.available_width();
                            changed |= ui
                                .add(
                                    egui::Slider::new(&mut theme.corner_radius, 0..=12)
                                        .show_value(false),
                                )
                                .changed();
                        });

                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(6.0);

                        ui.label(egui::RichText::new("BEHAVIOR").small().weak());
                        ui.add_space(2.0);
                        egui::Grid::new("theme-behavior")
                            .num_columns(2)
                            .min_col_width(96.0)
                            .spacing([12.0, 8.0])
                            .show(ui, |ui| {
                                // solid stripe fills float oddly on a
                                // transparent canvas — let people opt out
                                ui.label("Row stripes");
                                let acc = rgb(theme.accent);
                                let kn = rgb(theme.knob);
                                if toggle_switch(ui, &mut theme.row_stripes, true, acc, kn)
                                    .changed()
                                {
                                    changed = true;
                                }
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

                                // Windows 11's accent ring around the focused
                                // window; off = truly flush edges
                                ui.label("Focus outline");
                                if toggle_switch(ui, &mut theme.focus_outline, true, acc, kn)
                                    .changed()
                                {
                                    changed = true; // the shell applies it on change
                                }
                                ui.end_row();

                                // frosted vs sharp see-through; transparency
                                // itself is unconditional
                                ui.label("Blur");
                                if toggle_switch(ui, &mut theme.blur, true, acc, kn).changed() {
                                    changed = true; // the shell applies it on change
                                }
                                ui.end_row();
                            });

                        ui.add_space(8.0);
                        // canvas opacity in percent; 0 is safe — only the
                        // canvas fades, the widgets (this row included) stay solid
                        let mut pct =
                            (theme.window_opacity as f32 / 2.55).round().clamp(0.0, 100.0) as u8;
                        let before = pct;
                        ui.horizontal(|ui| {
                            ui.label("Opacity");
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.add(
                                        egui::DragValue::new(&mut pct)
                                            .range(0..=100)
                                            .suffix("%"),
                                    );
                                },
                            );
                        });
                        ui.scope(|ui| {
                            ui.spacing_mut().slider_width = ui.available_width();
                            ui.add(egui::Slider::new(&mut pct, 0..=100).show_value(false));
                        });
                        if pct != before {
                            theme.window_opacity = (pct as f32 * 2.55).round().min(255.0) as u8;
                            changed = true;
                        }

                        ui.add_space(8.0);
                        ui.label("Background image");
                        ui.add_space(2.0);
                        ui.horizontal(|ui| {
                            let picking = self_picker_active;
                            if ui
                                .add_enabled(!picking, egui::Button::new("Browse…"))
                                .clicked()
                            {
                                let (tx, rx) = std::sync::mpsc::channel();
                                picker_started = Some(rx);
                                std::thread::spawn(move || {
                                    let _ = tx.send(
                                        rfd::FileDialog::new()
                                            .set_title("Choose a background image")
                                            .add_filter(
                                                "Images",
                                                &["png", "jpg", "jpeg", "bmp", "webp"],
                                            )
                                            .pick_file(),
                                    );
                                });
                            }
                            if !theme.background_image.is_empty()
                                && ui.button("Clear").clicked()
                            {
                                theme.background_image.clear();
                                changed = true;
                            }
                        });
                        if !theme.background_image.is_empty() {
                            let name = std::path::Path::new(&theme.background_image)
                                .file_name()
                                .map(|n| n.to_string_lossy().into_owned())
                                .unwrap_or_else(|| theme.background_image.clone());
                            ui.label(egui::RichText::new(name).small().weak());
                        }
                    });
                });
                // the sheet's ONE border: a right-edge hairline riding the
                // slide — painted after the frame so the fill can't cover it
                let edge_x = screen.min.x - WIDTH * (1.0 - t) + WIDTH - 0.5;
                ui.painter().vline(
                    edge_x,
                    screen.min.y..=screen.max.y,
                    ui.visuals().window_stroke,
                );
            });
        if close {
            self.open = false;
        }
        self.presets_expanded = presets_expanded;
        if let Some(rx) = picker_started {
            self.picker = Some(rx);
        }
        // poll the off-thread dialog; keep frames coming while it's open
        if let Some(rx) = &self.picker {
            match rx.try_recv() {
                Ok(Some(path)) => {
                    theme.background_image = path.display().to_string();
                    changed = true;
                    self.picker = None;
                }
                Ok(None) => self.picker = None, // canceled
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    ui.ctx()
                        .request_repaint_after(std::time::Duration::from_millis(200));
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => self.picker = None,
            }
        }
        if changed {
            apply_style(ui.ctx(), theme); // live restyle, same frame
        }
        changed
    }
}
