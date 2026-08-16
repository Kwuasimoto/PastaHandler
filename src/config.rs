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
    pub fn init() -> Result<Config, AppError> {
        println!("Initializing configuration file");
        let sample = Config {
            snippets: vec![Snippet {
                label: "Starter Snippet".into(),
                text: "https://github.com/Kwuasimoto/PastaHandler".into(),
                hotkey: "ctrl+alt+Digit1".into()
            }]
        };
        return Ok(sample);
    }

    pub fn get_path() -> Result<PathBuf, AppError> {
        let appdata = std::env::var("APPDATA")
            .map_err(|_| AppError::Config("APPDATA env var not set".into()))?;
        let path = PathBuf::from(appdata).join("pastahandler").join("config.toml");
        let path_str = path.to_str().unwrap();
        println!("Path: {path_str}");
        Ok(path)
    }
}

impl ConfigFile {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn read(&self) -> Result<Config, AppError> {
        match std::fs::read_to_string(&self.path) {
            Ok(text) => Ok(toml::from_str(&text)?),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                let new_config = Config::init()?;
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
        std::fs::write(&tmp, contents)?;
        std::fs::rename(&tmp, &self.path)?;
        Ok(())
    }
}


