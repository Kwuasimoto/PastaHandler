//! The theme domain: what the app can look like. `Theme` is the persisted
//! appearance state (a field of `config::Config`), `PresetPalette` is the
//! palette-only contract presets apply, and `PRESETS` is the gallery.
//! Persistence itself lives in config.rs — this module changes when looks
//! change, that one when storage changes.

/// Curated theme; the settings UI derives every remaining shade from these so
/// any combination stays cohesive. serde(default) on the container: configs
/// saved before a field existed fall back to that field's default.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Theme {
    pub accent: [u8; 3],
    pub background: [u8; 3],
    pub text: [u8; 3],
    /// Resting border around inputs/buttons (hover/active borders derive from accent).
    pub border: [u8; 3],
    /// The sliding square inside the on/off toggles.
    pub knob: [u8; 3],
    pub corner_radius: u8,
    /// Remove the OS window border; the app draws its own title bar.
    pub borderless: bool,
    /// Optional background image path (empty = none), drawn cover-fit behind
    /// the settings content. Widget fills stay solid for readability.
    pub background_image: String,
    /// Canvas opacity, 0..=255 — affects only the color canvas BEHIND the
    /// background image (the image renders full-strength; its transparent
    /// regions reveal the faded canvas). Widgets stay solid, so 0 is safe:
    /// the UI floats on the desktop, visible and clickable.
    pub window_opacity: u8,
    /// Windows 11 draws an accent-colored focus border around every window;
    /// off makes the window truly flush (DWMWA_BORDER_COLOR = none).
    pub focus_outline: bool,
    /// Frost the see-through: on, the desktop behind the canvas is blurred
    /// (acrylic-style); off, it shows through sharp. Transparency itself is
    /// unconditional — opacity always reveals the desktop.
    pub blur: bool,
    /// Alternate-row shading in the snippet table. Solid fills read well on a
    /// solid canvas and like floating blocks on a transparent one.
    pub row_stripes: bool,
    /// Which bowl face the header and empty state wear.
    pub mascot: MascotStyle,
}

/// The mascot artwork variant. `Auto` picks ink by background luminance; the
/// rest override it. `Filled` is the color-traced bowl — it has no smile
/// variant, so the hover Easter egg rests while it's selected.
#[derive(Debug, Clone, Copy, PartialEq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MascotStyle {
    #[default]
    Auto,
    Dark,
    Light,
    Filled,
}

impl Default for Theme {
    fn default() -> Self {
        let p = DEFAULT_PALETTE;
        Self {
            accent: p.accent,
            background: p.background,
            text: p.text,
            border: p.border,
            knob: p.knob,
            corner_radius: p.corner_radius,
            borderless: true, // the app's own chrome is the intended look
            background_image: String::new(),
            window_opacity: 255,
            focus_outline: true,
            blur: true,
            row_stripes: true,
            mascot: MascotStyle::Auto,
        }
    }
}

impl Theme {
    /// This theme's palette slice — e.g. for "is this preset active" checks.
    pub fn palette(&self) -> PresetPalette {
        PresetPalette {
            accent: self.accent,
            background: self.background,
            text: self.text,
            border: self.border,
            knob: self.knob,
            corner_radius: self.corner_radius,
        }
    }

    /// Apply a preset. Only the palette changes — the type guarantees it.
    pub fn apply_palette(&mut self, p: &PresetPalette) {
        self.accent = p.accent;
        self.background = p.background;
        self.text = p.text;
        self.border = p.border;
        self.knob = p.knob;
        self.corner_radius = p.corner_radius;
    }
}

/// A preset is a PALETTE — five colors plus a corner radius — and nothing
/// more. The type IS the contract: applying a preset cannot touch window
/// behavior (borderless, opacity, blur, image, stripes) because a palette
/// has no such fields to touch.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PresetPalette {
    pub accent: [u8; 3],
    pub background: [u8; 3],
    pub text: [u8; 3],
    pub border: [u8; 3],
    pub knob: [u8; 3],
    pub corner_radius: u8,
}

/// The default look — single source for `Theme::default()` and the "Default"
/// preset card.
const DEFAULT_PALETTE: PresetPalette = PresetPalette {
    accent: [242, 183, 53],
    background: [27, 27, 27],
    text: [222, 222, 222],
    border: [58, 58, 58],
    knob: [235, 235, 235],
    corner_radius: 4,
};

/// The preset gallery. Researched palettes cite their grounding; the rest
/// state their reasoning.
pub const PRESETS: &[(&str, PresetPalette)] = &[
    ("Default", DEFAULT_PALETTE),
    // traditional Japanese blossom colors: warm petal wash (sakura-iro
    // lineage), deep plum text, rose accent, petal-soft corners
    ("Sakura", PresetPalette {
        accent: [178, 88, 118],
        background: [251, 236, 232],
        text: [86, 33, 53],
        border: [228, 180, 190],
        knob: [255, 249, 247],
        corner_radius: 5,
    }),
    // candy design convention: saturated bubblegum pink on candy-white,
    // grounded by deep plum text for contrast; gumdrop-round corners
    ("Candy", PresetPalette {
        accent: [255, 111, 181],
        background: [255, 233, 243],
        text: [90, 34, 70],
        border: [245, 184, 216],
        knob: [255, 255, 255],
        corner_radius: 10,
    }),
    // nebula palettes: electric violet ("plasma") glowing on deep-space
    // indigo, starlight text
    ("Cosmic", PresetPalette {
        accent: [141, 124, 238],
        background: [18, 14, 40],
        text: [232, 230, 244],
        border: [72, 60, 120],
        knob: [235, 232, 250],
        corner_radius: 6,
    }),
    // no canon exists — synthesized: gunmetal steel watchtower with an
    // amber warning-light accent, square military edges
    ("Sentinel", PresetPalette {
        accent: [255, 171, 64],
        background: [22, 27, 34],
        text: [206, 216, 226],
        border: [56, 68, 82],
        knob: [228, 234, 240],
        corner_radius: 2,
    }),
    // per spec: Gears + God of War = near-black ash, blood-crimson accent
    ("GoW", PresetPalette {
        accent: [196, 30, 35],
        background: [16, 13, 13],
        text: [214, 205, 200],
        border: [72, 46, 46],
        knob: [226, 216, 212],
        corner_radius: 3,
    }),
    // Master Chief MJOLNIR green #507D2A on UNSC green-black; the toggle
    // knob is the visor gold
    ("Halo", PresetPalette {
        accent: [80, 125, 42],
        background: [18, 24, 16],
        text: [212, 220, 208],
        border: [58, 72, 48],
        knob: [255, 196, 80],
        corner_radius: 4,
    }),
    // Riot's published brand set: gold #C89B3C on dark navy #0A1428,
    // hextech cream #F0E6D2 text, dark-gold #785A28 borders
    ("League", PresetPalette {
        accent: [200, 155, 60],
        background: [10, 20, 40],
        text: [240, 230, 210],
        border: [120, 90, 40],
        knob: [240, 230, 210],
        corner_radius: 4,
    }),
    // the fruit itself: ripe red on cream-blush flesh, deep berry text
    ("Strawberry", PresetPalette {
        accent: [224, 66, 84],
        background: [255, 243, 240],
        text: [108, 26, 38],
        border: [244, 190, 190],
        knob: [255, 251, 245],
        corner_radius: 8,
    }),
    // espresso lineage with latte-cream text and a caramel accent (coffee
    // palette consensus: espresso / cream / caramel)
    ("Coffee", PresetPalette {
        accent: [216, 160, 120],
        background: [43, 30, 22],
        text: [244, 238, 228],
        border: [94, 70, 54],
        knob: [246, 241, 233],
        corner_radius: 6,
    }),
];

#[cfg(test)]
mod tests {
    use super::*;

    /// Guards future palette edits: every preset must keep readable contrast
    /// between text and background, and names must stay unique.
    #[test]
    fn presets_are_readable_and_uniquely_named() {
        let luma = |c: [u8; 3]| 0.299 * c[0] as f32 + 0.587 * c[1] as f32 + 0.114 * c[2] as f32;
        let mut names = std::collections::HashSet::new();
        for (name, palette) in PRESETS {
            assert!(names.insert(name), "duplicate preset name: {name}");
            let delta = (luma(palette.text) - luma(palette.background)).abs();
            assert!(
                delta >= 120.0,
                "preset '{name}' text/background contrast too low (luma delta {delta:.0})"
            );
        }
    }
}
