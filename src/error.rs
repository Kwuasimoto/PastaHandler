#[derive(Debug)]
pub enum AppError {
    Io(std::io::Error),
    TomlParse(toml::de::Error),
    TomlWrite(toml::ser::Error),
    Hotkey(global_hotkey::Error),
    Clipboard(arboard::Error),
    Config(String),
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

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::Io(e) => write!(f, "file error: {e}"),
            AppError::TomlParse(e) => write!(f, "config serialize error: {e}"),
            AppError::TomlWrite(e) => write!(f, "config write error: {e}"),
            AppError::Hotkey(e) => write!(f, "hotkey error: {e}"),
            AppError::Clipboard(e) => write!(f, "clipboard error: {e}"),
            AppError::Config(e) => write!(f, "config error: {e}"),
        }
    }
}