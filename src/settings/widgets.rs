//! Leaf widgets shared by 2+ regions. Deliberately the only residents until a
//! second shared widget exists (three-instances rule applies to homes too).

use eframe::egui;

/// A small animated on/off switch (egui ships no built-in). Accent when on;
/// grayed and inert when `enabled` is false.
pub fn toggle_switch(
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
