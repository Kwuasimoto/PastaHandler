use std::collections::{HashMap, HashSet};

use global_hotkey::{GlobalHotKeyManager, hotkey::HotKey};

use crate::{config::Config, error::AppError};

pub struct Hotkeys {
    hotkey_manager: GlobalHotKeyManager,
    registered: Vec<HotKey>,
}

impl Hotkeys {
    pub fn new() -> Result<Self, AppError> {
        Ok(Self {
            hotkey_manager: GlobalHotKeyManager::new()?,
            registered: Vec::new(),
        })
    }

    /// Idempotent: replaces the previous registration set, so first registration
    /// and config reload are the same call. The whole config is validated before
    /// any existing registration is touched — a bad config returns Err while the
    /// old hotkeys are still live.
    pub fn register_all(&mut self, config: &Config) -> Result<HashMap<u32, usize>, AppError> {
        // pass 1: parse + duplicate-check everything; no side effects
        let parsed = parse_all(config)?;

        // pass 2: unregister the old set; individual failures are ignored so one
        // stuck key can't poison every future reload
        for hk in self.registered.drain(..) {
            let _ = self.hotkey_manager.unregister(hk);
        }

        // pass 3: register the new set
        let mut map = HashMap::new();
        for (hotkey, i) in parsed {
            self.hotkey_manager.register(hotkey)?;
            self.registered.push(hotkey);
            map.insert(hotkey.id(), i);
        }
        Ok(map)
    }
}

/// Shared validation: parse + duplicate-check every hotkey in the config.
/// No side effects — the settings UI calls this on drafts, and it is pass 1
/// of `register_all`.
pub fn parse_all(config: &Config) -> Result<Vec<(HotKey, usize)>, AppError> {
    let mut parsed = Vec::new();
    let mut seen = HashSet::new();
    for (i, snippet) in config.snippets.iter().enumerate() {
        let hotkey: HotKey = snippet.hotkey.parse().map_err(|_| {
            AppError::Config(format!("bad hotkey '{}' on '{}'", snippet.hotkey, snippet.label))
        })?;
        if !seen.insert(hotkey.id()) {
            return Err(AppError::Config(format!("duplicate hotkey '{}'", snippet.hotkey)));
        }
        parsed.push((hotkey, i));
    }
    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Snippet;

    fn config_with(hotkeys: &[&str]) -> Config {
        Config {
            snippets: hotkeys
                .iter()
                .enumerate()
                .map(|(i, hk)| Snippet {
                    label: format!("s{i}"),
                    text: format!("text{i}"),
                    hotkey: (*hk).into(),
                })
                .collect(),
            ..Default::default()
        }
    }

    #[test]
    fn parse_all_accepts_valid_hotkeys() {
        let config = config_with(&["ctrl+alt+Digit1", "ctrl+shift+KeyQ", "F5"]);
        let parsed = parse_all(&config).expect("valid hotkeys should parse");
        assert_eq!(parsed.len(), 3);
        assert_eq!(parsed[2].1, 2); // indices preserved
    }

    #[test]
    fn parse_all_rejects_garbage_with_context() {
        let config = config_with(&["ctrl+alt+Banana"]);
        let err = parse_all(&config).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("ctrl+alt+Banana"), "error names the combo: {msg}");
        assert!(msg.contains("s0"), "error names the snippet: {msg}");
    }

    #[test]
    fn parse_all_rejects_duplicates() {
        let config = config_with(&["ctrl+alt+Digit1", "CTRL+ALT+Digit1"]); // case-insensitive dup
        let err = parse_all(&config).unwrap_err();
        assert!(err.to_string().contains("duplicate"), "{err}");
    }
}
