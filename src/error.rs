use std::fmt::Debug;

#[derive(Debug)]
pub enum AppError {
    Io(std::io::Error),
    TomlParse(toml::de::Error),
    TomlWrite(toml::ser::Error),
    Image(image::ImageError),
    BadIcon(tray_icon::BadIcon),
    TrayIcon(tray_icon::Error),
    TrayMenu(tray_icon::menu::Error),
    Hotkey(global_hotkey::Error),
    Clipboard(arboard::Error),
    /// Config-content problems in the user's own data: bad hotkey strings,
    /// duplicate active combos, a missing APPDATA. The message is user-facing.
    Config(String),
    /// The settings window itself failed to launch or run (eframe/GPU layer).
    Gui(String),
}

pub type Result<T> = std::result::Result<T, AppError>;

impl From<arboard::Error> for AppError {
    fn from(e: arboard::Error) -> Self {
        AppError::Clipboard(e)
    }
}

impl From<global_hotkey::Error> for AppError {
    fn from(e: global_hotkey::Error) -> Self {
        AppError::Hotkey(e)
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::Io(e)
    }
}

impl From<toml::ser::Error> for AppError {
    fn from(e: toml::ser::Error) -> Self {
        AppError::TomlWrite(e)
    }
}

impl From<toml::de::Error> for AppError {
    fn from(e: toml::de::Error) -> Self {
        AppError::TomlParse(e)
    }
}

impl From<image::ImageError> for AppError {
    fn from(e: image::ImageError) -> Self {
        AppError::Image(e)
    }
}

impl From<tray_icon::Error> for AppError {
    fn from(e: tray_icon::Error) -> Self {
        AppError::TrayIcon(e)
    }
}

impl From<tray_icon::menu::Error> for AppError {
    fn from(e: tray_icon::menu::Error) -> Self {
        AppError::TrayMenu(e)
    }
}

impl From<tray_icon::BadIcon> for AppError {
    fn from(e: tray_icon::BadIcon) -> Self {
        AppError::BadIcon(e)
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::Io(e) => write!(f, "file error: {e}"),
            AppError::TomlParse(e) => write!(f, "config parse error: {e}"),
            AppError::TomlWrite(e) => write!(f, "config write error: {e}"),
            AppError::Image(e) => write!(f, "image error: {e}"),
            AppError::BadIcon(e) => write!(f, "bad icon error: {e}"),
            AppError::TrayIcon(e) => write!(f, "tray icon error: {e}"),
            AppError::TrayMenu(e) => write!(f, "tray error: {e}"),
            AppError::Hotkey(e) => write!(f, "hotkey error: {e}"),
            AppError::Clipboard(e) => write!(f, "clipboard error: {e}"),
            AppError::Config(e) => write!(f, "config error: {e}"),
            AppError::Gui(e) => write!(f, "settings window error: {e}"),
        }
    }
}