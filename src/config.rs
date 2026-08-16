use std::path::PathBuf;

use crate::{error::AppError};

pub struct ConfigFile { path: PathBuf }

#[derive(Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Config { pub snippets: Vec<Snippet> }

#[derive(PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Snippet {
    pub label: String,
    pub text: String,
    pub hotkey: String
}

impl Config {
    pub fn get_path() -> Result<PathBuf, AppError> {
        let appdata = std::env::var("APPDATA")
            .map_err(|_| AppError::Config("APPDATA env var not set".into()))?;
        Ok(PathBuf::from(appdata).join("tnp").join("config.toml"))
    }
}

impl ConfigFile {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn read(&self) -> Result<Config, AppError> {
        match std::fs::read_to_string(&self.path) {
            Ok(text) => Ok(toml::from_str(&text)?),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Config::default()),
            Err(e) => Err(e.into())
        }
    }
    
    pub fn write(&self, config: &Config) -> Result<(), AppError> {
        let contents = toml::to_string_pretty(config)?;
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = self.path.with_extension("toml.tmp");
        std::fs::write(&tmp, contents)?;
        std::fs::rename(&tmp, &self.path)?;
        Ok(())
    }
}


