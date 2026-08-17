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
# PastaHandler config
# Hotkey format: \"ctrl+alt+Digit1\" — modifiers: ctrl, alt, shift, super (case-insensitive)
# Full list of key names (Digit1, KeyQ, F5, ...):
# https://docs.rs/global-hotkey/latest/global_hotkey/hotkey/enum.Code.html
";

impl Config {
    pub fn init() -> Config {
        println!("Initializing configuration file");
        Config {
            open_settings_on_launch: true,
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
        let path = PathBuf::from(appdata).join("pastahandler").join("config.toml");
        println!("Path: {}", path.display());
        Ok(path)
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

    #[test]
    fn read_missing_file_seeds_sample_with_header() {
        let file = temp_config("pastahandler-test-seed.toml");
        let _ = std::fs::remove_file(file.path());
        let config = file.read().expect("missing file seeds a sample");
        assert!(!config.snippets.is_empty());
        let text = std::fs::read_to_string(file.path()).expect("file was created");
        assert!(text.starts_with("# PastaHandler config"), "header comment present");
        let _ = std::fs::remove_file(file.path());
    }
}
