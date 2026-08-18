//! Everything that turns a `Theme` into pixels: color math, the egui style
//! application, font installation, and theme-derived assets.
//! Future ask "new theme derivation rule" lands here.

use eframe::egui;

use crate::config::Theme;

pub fn rgb(c: [u8; 3]) -> egui::Color32 {
    egui::Color32::from_rgb(c[0], c[1], c[2])
}

/// Lighten (or darken with a negative `add`) a theme color by a fixed amount.
pub fn lift(c: [u8; 3], add: i16) -> egui::Color32 {
    let l = |v: u8| (v as i16 + add).clamp(0, 255) as u8;
    egui::Color32::from_rgb(l(c[0]), l(c[1]), l(c[2]))
}

pub fn luma3(c: [u8; 3]) -> f32 {
    0.299 * c[0] as f32 + 0.587 * c[1] as f32 + 0.114 * c[2] as f32
}

/// Black or white, whichever contrasts against the given color — for text
/// sitting on accent-filled surfaces.
pub fn on_color(c: egui::Color32) -> egui::Color32 {
    if luma3([c.r(), c.g(), c.b()]) > 140.0 {
        egui::Color32::from_rgb(20, 20, 20)
    } else {
        egui::Color32::WHITE
    }
}

/// Blend `a` toward `b` by `t` (0..1).
pub fn mix(a: [u8; 3], b: [u8; 3], t: f32) -> egui::Color32 {
    let m = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t) as u8;
    egui::Color32::from_rgb(m(a[0], b[0]), m(a[1], b[1]), m(a[2], b[2]))
}

/// Multiply a theme color's brightness.
pub fn scale(c: [u8; 3], f: f32) -> egui::Color32 {
    let s = |v: u8| ((v as f32 * f) as i16).clamp(0, 255) as u8;
    egui::Color32::from_rgb(s(c[0]), s(c[1]), s(c[2]))
}

/// Per-frame owned colors shared by regions — computed AFTER the theme window
/// each frame, so theme edits recolor the same frame. Owned `Color32`s (never
/// references into the theme) are what keep region borrows disjoint.
pub struct Palette {
    pub accent: egui::Color32,
    pub knob: egui::Color32,
    /// Inactive-row text, ghost icons, and the resting resize grip.
    pub dim: egui::Color32,
    /// Alternate-row shading in the snippet table (off for transparent looks).
    pub row_stripes: bool,
}

impl Palette {
    pub fn from_theme(theme: &Theme) -> Self {
        Self {
            accent: rgb(theme.accent),
            knob: rgb(theme.knob),
            dim: scale(theme.text, 0.52),
            row_stripes: theme.row_stripes,
        }
    }
}

/// Paint the window canvas: the theme background at the chosen opacity, then
/// the optional background image cover-fitted over it (scaled to fill,
/// centered, cropped at the edges — never stretched). Runs before every other
/// region; the central panel's own fill is transparent so this shows through.
/// Widgets paint solid on top — only the canvas is glass; the DWM accent
/// policy (win32::enable_glass) composites the desktop behind its alpha.
pub fn paint_background(ui: &egui::Ui, theme: &Theme) {
    let rect = ui.ctx().content_rect();
    let alpha = theme.window_opacity as f32 / 255.0;
    // the canvas's alpha stays open unconditionally — DWM fills it with the
    // desktop behind (frosted or sharp per the Blur toggle, win32::set_blur)
    ui.painter()
        .rect_filled(rect, 0, rgb(theme.background).gamma_multiply(alpha));
    if theme.background_image.is_empty() {
        return;
    }
    // three slashes, not two: file://X treats X as a UNC hostname on Windows;
    // file:///X is a local path (egui_extras file_loader::convert_uri_to_path)
    let img = egui::Image::new(format!("file:///{}", theme.background_image))
        .tint(egui::Color32::WHITE.gamma_multiply(alpha));
    match img.load_for_size(ui.ctx(), rect.size()) {
        Ok(egui::load::TexturePoll::Ready { texture }) => {
            let scale = (rect.width() / texture.size.x).max(rect.height() / texture.size.y);
            let cover = egui::Rect::from_center_size(rect.center(), texture.size * scale);
            img.paint_at(ui, cover); // the ui's clip crops the overflow — cover-fit
        }
        Ok(egui::load::TexturePoll::Pending { .. }) => {
            ui.ctx().request_repaint(); // first frame after picking: try again
        }
        Err(_) => {} // bad path/format: the color canvas stands; drawer shows the file name
    }
}

/// The mascot adapts to the theme: dark ink on light backgrounds, light ink
/// on dark. Computed after the theme drawer each frame, so a background edit
/// restyles the mascot the same frame.
pub fn mascot_for(theme: &Theme) -> egui::ImageSource<'static> {
    if luma3(theme.background) > 128.0 {
        egui::include_image!("../../assets/icon-line-dark.svg")
    } else {
        egui::include_image!("../../assets/icon-line-light.svg")
    }
}

/// The hover face: same ink as `mascot_for`, mouth open in a smile, blushing.
/// The header swaps it in while the pointer rests on the mascot.
pub fn mascot_smile_for(theme: &Theme) -> egui::ImageSource<'static> {
    if luma3(theme.background) > 128.0 {
        egui::include_image!("../../assets/icon-line-dark-smile.svg")
    } else {
        egui::include_image!("../../assets/icon-line-light-smile.svg")
    }
}

/// Widget styling derived entirely from the curated theme: two colors and a
/// radius in, every shade out — so any user combination stays cohesive.
pub fn apply_style(ctx: &egui::Context, theme: &Theme) {
    let accent = rgb(theme.accent);
    let bg = theme.background;
    let radius = egui::CornerRadius::same(theme.corner_radius);
    // derivation direction depends on the theme's brightness: on dark themes
    // raised surfaces get lighter; on light themes they get darker — and
    // inputs go the opposite way (well on dark, paper on light)
    let is_light = luma3(bg) > 128.0;
    let step = |amount: i16| if is_light { lift(bg, -amount) } else { lift(bg, amount) };
    let input_bg = if is_light { mix(bg, [255, 255, 255], 0.7) } else { scale(bg, 0.6) };
    ctx.all_styles_mut(|style| {
        style.spacing.button_padding = egui::vec2(12.0, 5.0);
        style.spacing.item_spacing = egui::vec2(10.0, 8.0);
        style.spacing.interact_size.y = 26.0;

        let v = &mut style.visuals;
        v.panel_fill = rgb(bg);
        v.window_fill = step(8); // floating windows sit slightly off the panel
        v.window_stroke = egui::Stroke::new(1.0, rgb(theme.border)); // tooltip/popup outlines
        v.window_shadow = egui::Shadow {
            offset: [0, 4],
            blur: 14,
            spread: 0,
            color: egui::Color32::from_black_alpha(50),
        };
        v.popup_shadow = egui::Shadow {
            offset: [0, 3],
            blur: 10,
            spread: 0,
            color: egui::Color32::from_black_alpha(40),
        };
        v.faint_bg_color = step(6); // table stripes follow the theme
        v.text_edit_bg_color = Some(input_bg);
        v.selection.bg_fill = accent.linear_multiply(0.25); // text selection wash
        v.selection.stroke = egui::Stroke::new(1.0, accent); // focused-input ring
        v.text_cursor.stroke.color = accent; // the caret itself
        v.slider_trailing_fill = true; // traveled slider track takes selection/accent color

        for w in [
            &mut v.widgets.noninteractive,
            &mut v.widgets.inactive,
            &mut v.widgets.hovered,
            &mut v.widgets.active,
            &mut v.widgets.open,
        ] {
            w.corner_radius = radius;
        }
        // text color per widget state — this is what heading/strong/weak actually
        // read (an override_text_color would flatten their distinctions instead)
        let text = rgb(theme.text);
        v.widgets.noninteractive.fg_stroke.color = text; // labels, footer
        v.widgets.inactive.fg_stroke.color = text; //       resting buttons/inputs
        v.widgets.hovered.fg_stroke.color = lift(theme.text, 20);
        v.widgets.active.fg_stroke.color = lift(theme.text, 33); // also "strong"/headings
        v.widgets.open.fg_stroke.color = text;

        v.widgets.inactive.weak_bg_fill = step(11);
        v.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, rgb(theme.border));
        v.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, rgb(theme.border)); // separators
        v.widgets.hovered.weak_bg_fill = step(21);
        v.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, accent.linear_multiply(0.6));
        v.widgets.active.weak_bg_fill = step(28);
        v.widgets.active.bg_stroke = egui::Stroke::new(1.0, accent);
    });
}

/// Replace egui's default typeface with Nunito for all proportional text.
pub fn install_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "nunito".into(),
        egui::FontData::from_static(include_bytes!("../../assets/fonts/Nunito-Regular.ttf"))
            .into(),
    );
    fonts
        .families
        .get_mut(&egui::FontFamily::Proportional)
        .expect("proportional family exists")
        .insert(0, "nunito".into());
    ctx.set_fonts(fonts);
}
