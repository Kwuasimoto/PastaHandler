//! The header deck region: drag strip + caption controls (borderless), mascot,
//! primary Add button, theme swatch button, and the closing inset hairline.
//! Stateless — events exit via HeaderOutput. Future ask "change the header"
//! touches this file only.

use eframe::egui;

use super::chrome::{caption_button, CaptionIcon};
use super::style::{on_color, rgb};
use crate::theme::Theme;

#[must_use]
pub struct HeaderOutput {
    /// The app pushes the blank Snippet. Deliberately does NOT commit —
    /// a blank row is only persisted on its first edit/toggle.
    pub add_snippet: bool,
    /// The app flips ThemeWindow.open (same-frame open preserved).
    pub toggle_theme: bool,
}

pub fn show(
    ui: &mut egui::Ui,
    theme: &Theme,
    mascot: egui::ImageSource<'static>,
    mascot_smile: egui::ImageSource<'static>,
) -> HeaderOutput {
    let mut out = HeaderOutput { add_snippet: false, toggle_theme: false };

    // ONE deck: mascot left (+ small window title when chromeless), actions
    // right, caption controls rightmost. Whole deck drags in borderless mode.
    // ORDER MATTERS: the drag interact registers BEFORE the row's widgets so
    // buttons stay clickable on top of the drag region.
    if theme.borderless {
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

    ui.horizontal(|ui| {
        // hover Easter egg: hit-test the rect BEFORE painting so the smile
        // appears the same frame the pointer arrives — no stored state, no lag
        let (rect, _) = ui.allocate_exact_size(egui::vec2(63.0, 52.0), egui::Sense::hover());
        let face = if ui.rect_contains_pointer(rect) { mascot_smile } else { mascot };
        egui::Image::new(face).paint_at(ui, rect);
        if theme.borderless {
            // stands in for the OS title bar text that decorations provide
            ui.add_space(4.0);
            ui.label(egui::RichText::new(crate::win32::SETTINGS_WINDOW_TITLE).weak());
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if theme.borderless {
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
            let header_accent = rgb(theme.accent);
            let add = egui::Button::new(
                egui::RichText::new("+  Add snippet").color(on_color(header_accent)),
            )
            .fill(header_accent);
            if ui.add(add).clicked() {
                out.add_snippet = true;
            }
            if theme_swatch_button(ui, theme).clicked() {
                out.toggle_theme = true;
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

    out
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
