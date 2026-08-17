#![windows_subsystem = "windows"] // comment out when you need println!/eprintln! debugging

pub mod clipboard;
pub mod config;
pub mod error;
pub mod hotkeys;
pub mod resident;
pub mod settings;
pub mod tray;

use crate::{
    config::{Config, ConfigFile},
    error::AppError,
};

fn dispatch() -> Result<(), AppError> {
    let config_file = ConfigFile::new(Config::build_path()?);
    if std::env::args().nth(1).as_deref() == Some("--settings") {
        settings::run(config_file)
    } else {
        resident::run(config_file)
    }
}

fn main() {
    if let Err(e) = dispatch() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
