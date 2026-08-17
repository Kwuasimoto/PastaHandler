//! Borderless-window chrome primitives: caption buttons and the corner resize
//! grip. Future ask "new window mode" lands here (plus one compose line).

use eframe::egui;

use super::style::Palette;

#[derive(Clone, Copy, PartialEq)]
pub enum CaptionIcon {
    Minimize,
    Maximize,
    Restore,
    Close,
}

/// A title-bar caption button: painter-drawn icon (no font-glyph gambling),
/// backplate on hover — the close button's backplate is the Windows-convention red.
pub fn caption_button(ui: &mut egui::Ui, icon: CaptionIcon, danger: bool) -> egui::Response {
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

/// Borderless windows lose the OS resize edges — the classic corner grip in
/// their place. Call last so it paints over anything in the corner.
pub fn resize_grip(ui: &mut egui::Ui, palette: &Palette) {
    let screen = ui.ctx().content_rect(); // egui 0.36: was screen_rect
    let grip = 20.0;
    let grip_rect = egui::Rect::from_min_max(screen.max - egui::vec2(grip, grip), screen.max);
    let resp = ui
        .interact(grip_rect, egui::Id::new("resize-grip-se"), egui::Sense::drag())
        .on_hover_cursor(egui::CursorIcon::ResizeNwSe)
        .on_hover_text("Drag to resize the window");
    if resp.drag_started_by(egui::PointerButton::Primary) {
        ui.ctx().send_viewport_cmd(egui::ViewportCommand::BeginResize(
            egui::ResizeDirection::SouthEast,
        ));
    }
    let grip_color = if resp.hovered() { palette.accent } else { palette.dim };
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
