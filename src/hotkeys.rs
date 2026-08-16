use std::{collections::{HashMap, HashSet}};

use global_hotkey::{GlobalHotKeyManager, hotkey::HotKey};

use crate::{config::Config, error::AppError};

pub struct Hotkeys { 
    hotkey_manager: GlobalHotKeyManager
}

impl Hotkeys {
    pub fn new() -> Result<Self, AppError> {
        Ok(Self { hotkey_manager: GlobalHotKeyManager::new()? })
    }

    pub fn register_all(&self, config: &Config) -> Result<HashMap<u32, usize>, AppError> {
        let mut map = HashMap::new();
        let mut seen = HashSet::new();

        for (i, snippet) in config.snippets.iter().enumerate() {
            let hotkey: HotKey = snippet.hotkey.parse().map_err(|_| {
                AppError::Config(format!("bad hotkey '{}' on '{}'", snippet.hotkey, snippet.label))
            })?;
            if !seen.insert(hotkey.id()) {
                return Err(AppError::Config(format!("duplicate hotkey '{}'", snippet.hotkey)));
            }
            self.hotkey_manager.register(hotkey)?;
            map.insert(hotkey.id(), i);
        }
        Ok(map)
    }
}