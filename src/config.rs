//! The config file: domain structs (Config/Snippet/Theme), TOML persistence
//! with atomic writes, and first-run seeding. This file on disk is also the
//! IPC channel between the resident and settings processes.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::error::AppError;

pub struct ConfigFile { 
    header: String, 
    path: PathBuf 
}

#[derive(Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Config {
    pub snippets: Vec<Snippet>,
    #[serde(default)]
    pub open_settings_on_launch: bool,
    #[serde(default)]
    pub theme: Theme,
}

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
    /// Canvas opacity, 0..=255: the background (color + image) turns to glass
    /// while the widgets stay solid. 0 = fully clear canvas — the UI floats on
    /// the desktop, still visible and clickable, so no lockout is possible.
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
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            accent: [242, 183, 53],
            background: [27, 27, 27],
            text: [222, 222, 222],
            border: [58, 58, 58],
            knob: [235, 235, 235],
            corner_radius: 4,
            borderless: false,
            background_image: String::new(),
            window_opacity: 255,
            focus_outline: true,
            blur: true,
            row_stripes: true,
        }
    }
}

impl Theme {
    /// Sakura preset — grounded in traditional Japanese blossom colors:
    /// a warm petal background (sakura-iro lineage), deep plum text, rose accent.
    pub fn sakura() -> Self {
        Self {
            accent: [178, 88, 118],      // deep blossom rose
            background: [251, 236, 232], // warm petal wash
            text: [86, 33, 53],          // plum (traditional pairing)
            border: [228, 180, 190],     // soft petal edge
            knob: [255, 249, 247],       // warm white
            corner_radius: 5,            // petal-soft
            borderless: false,
            background_image: String::new(),
            window_opacity: 255,
            focus_outline: true,
            blur: true,
            row_stripes: true,
        }
    }

    /// The preset gallery. Presets are PALETTES: the drawer applies only the
    /// five colors + corner radius from these — window behavior (borderless,
    /// opacity, blur, image, stripes) always stays the user's. Researched
    /// palettes cite their grounding; the rest state their reasoning.
    pub fn presets() -> Vec<(&'static str, Theme)> {
        let d = Theme::default;
        vec![
            ("Default", d()),
            ("Sakura", Theme::sakura()),
            // candy design convention: saturated bubblegum pink on candy-white,
            // grounded by deep plum text for contrast (palette-guide consensus)
            ("Candy", Theme {
                accent: [255, 111, 181],
                background: [255, 233, 243],
                text: [90, 34, 70],
                border: [245, 184, 216],
                knob: [255, 255, 255],
                corner_radius: 10, // gumdrop-round
                ..d()
            }),
            // nebula palettes: electric violet ("plasma") glowing on deep-space
            // indigo, starlight text
            ("Cosmic", Theme {
                accent: [141, 124, 238],
                background: [18, 14, 40],
                text: [232, 230, 244],
                border: [72, 60, 120],
                knob: [235, 232, 250],
                corner_radius: 6,
                ..d()
            }),
            // no canon exists — synthesized: gunmetal steel watchtower with an
            // amber warning-light accent, square military edges
            ("Sentinel", Theme {
                accent: [255, 171, 64],
                background: [22, 27, 34],
                text: [206, 216, 226],
                border: [56, 68, 82],
                knob: [228, 234, 240],
                corner_radius: 2,
                ..d()
            }),
            // per spec: Gears + God of War = near-black ash, blood-crimson accent
            ("GoW", Theme {
                accent: [196, 30, 35],
                background: [16, 13, 13],
                text: [214, 205, 200],
                border: [72, 46, 46],
                knob: [226, 216, 212],
                corner_radius: 3,
                ..d()
            }),
            // Master Chief MJOLNIR green #507D2A on UNSC green-black; the
            // toggle knob is the visor gold
            ("Halo", Theme {
                accent: [80, 125, 42],
                background: [18, 24, 16],
                text: [212, 220, 208],
                border: [58, 72, 48],
                knob: [255, 196, 80],
                corner_radius: 4,
                ..d()
            }),
            // Riot's published brand set: gold #C89B3C on dark navy #0A1428,
            // hextech cream #F0E6D2 text, dark-gold #785A28 borders
            ("League", Theme {
                accent: [200, 155, 60],
                background: [10, 20, 40],
                text: [240, 230, 210],
                border: [120, 90, 40],
                knob: [240, 230, 210],
                corner_radius: 4,
                ..d()
            }),
            // the fruit itself: ripe red on cream-blush flesh, deep berry text
            ("Strawberry", Theme {
                accent: [224, 66, 84],
                background: [255, 243, 240],
                text: [108, 26, 38],
                border: [244, 190, 190],
                knob: [255, 251, 245],
                corner_radius: 8,
                ..d()
            }),
            // espresso #2B1E16 lineage with latte-cream text and a caramel
            // accent (coffee palette consensus: espresso / cream / caramel)
            ("Coffee", Theme {
                accent: [216, 160, 120],
                background: [43, 30, 22],
                text: [244, 238, 228],
                border: [94, 70, 54],
                knob: [246, 241, 233],
                corner_radius: 6,
                ..d()
            }),
        ]
    }
}

#[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Snippet {
    pub text: String,
    pub hotkey: String,
    /// Only active snippets register their hotkey. Defaults true so configs
    /// from before this field existed keep working after upgrade.
    #[serde(default = "default_active")]
    pub active: bool,
}

fn default_active() -> bool {
    true
}

const HEADER: &str = "\
# Pasta Handler config
# Hotkey format: \"ctrl+alt+Digit1\" — modifiers: ctrl, alt, shift, super (case-insensitive)
# Full list of key names (Digit1, KeyQ, F5, ...):
# https://docs.rs/global-hotkey/latest/global_hotkey/hotkey/enum.Code.html
";

impl Config {
    pub fn init() -> Config {
        Config {
            open_settings_on_launch: true,
            theme: Theme::default(),
            snippets: vec![Snippet {
                text: "https://github.com/Kwuasimoto/PastaHandler".into(),
                hotkey: "ctrl+alt+Digit1".into(),
                active: true,
            }]
        }
    }

    pub fn build_path() -> Result<PathBuf, AppError> {
        let appdata = std::env::var("APPDATA")
            .map_err(|_| AppError::Config("APPDATA env var not set".into()))?;
        Ok(PathBuf::from(appdata).join("pastahandler").join("config.toml"))
    }
}
impl ConfigFile {
    pub fn new(path: PathBuf) -> Self {
        Self { path, header: HEADER.into() }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// None when the file is missing or unreadable — callers compare Options,
    /// they don't handle errors here.
    pub fn mtime(&self) -> Option<SystemTime> {
        std::fs::metadata(&self.path).and_then(|m| m.modified()).ok()
    }

    pub fn read(&self) -> Result<Config, AppError> {
        match std::fs::read_to_string(&self.path) {
            Ok(text) => Ok(toml::from_str(&text)?),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                crate::logging::log_event("config missing — seeding the sample config");
                let new_config = Config::init();
                self.write(&new_config)?;
                Ok(new_config)
            },
            Err(e) => Err(e.into())
        }
    }

    pub fn write(&self, config: &Config) -> Result<(), AppError> {
        let contents = toml::to_string_pretty(config)?;
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = self.path.with_extension("toml.tmp");
        std::fs::write(&tmp, format!("{}\n{}", self.header, contents))?;
        std::fs::rename(&tmp, &self.path)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_config(name: &str) -> ConfigFile {
        ConfigFile::new(std::env::temp_dir().join(name))
    }

    #[test]
    fn round_trip_preserves_config() {
        let file = temp_config("pastahandler-test-roundtrip.toml");
        let original = Config {
            open_settings_on_launch: true,
            theme: Theme::default(),
            snippets: vec![Snippet {
                text: "T".into(),
                hotkey: "ctrl+alt+Digit1".into(),
                active: true,
            }],
        };
        file.write(&original).expect("write");
        let loaded = file.read().expect("read");
        assert_eq!(loaded, original);
        let _ = std::fs::remove_file(file.path());
    }

    /// Guards future palette edits: every preset must keep readable contrast
    /// between text and background, and names must stay unique.
    #[test]
    fn presets_are_readable_and_uniquely_named() {
        let luma = |c: [u8; 3]| 0.299 * c[0] as f32 + 0.587 * c[1] as f32 + 0.114 * c[2] as f32;
        let mut names = std::collections::HashSet::new();
        for (name, theme) in Theme::presets() {
            assert!(names.insert(name), "duplicate preset name: {name}");
            let delta = (luma(theme.text) - luma(theme.background)).abs();
            assert!(
                delta >= 120.0,
                "preset '{name}' text/background contrast too low (luma delta {delta:.0})"
            );
        }
    }

    #[test]
    fn read_missing_file_seeds_sample_with_header() {
        let file = temp_config("pastahandler-test-seed.toml");
        let _ = std::fs::remove_file(file.path());
        let config = file.read().expect("missing file seeds a sample");
        assert!(!config.snippets.is_empty());
        let text = std::fs::read_to_string(file.path()).expect("file was created");
        assert!(text.starts_with("# Pasta Handler config"), "header comment present");
        let _ = std::fs::remove_file(file.path());
    }
}
