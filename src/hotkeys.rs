//! Global hotkey registration via Win32 `RegisterHotKey` (the narrow API: the
//! OS delivers only the combos we registered — this process never observes any
//! other keystroke). See COMPLIANCE.md for why that distinction matters.

use std::collections::{HashMap, HashSet};

use global_hotkey::{GlobalHotKeyManager, hotkey::HotKey};

use crate::{config::Config, error::AppError};

/// Owns the OS-level registrations. RAII matters here: dropping this (or the
/// manager inside it) is what unregisters the combos — keep it alive as long
/// as the hotkeys should work.
pub struct Hotkeys {
    manager: GlobalHotKeyManager,
    /// What we currently hold registered with the OS — the set to release on
    /// the next `register_all` (drop alone is NOT documented to release combos).
    registered: Vec<HotKey>,
}

impl Hotkeys {
    pub fn new() -> Result<Self, AppError> {
        Ok(Self {
            manager: GlobalHotKeyManager::new()?,
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
            let _ = self.manager.unregister(hk);
        }

        // pass 3: register the new set; the map's key is `HotKey::id()` (a hash
        // of the combo) — the same id the resident receives in hotkey events
        let mut map = HashMap::new();
        for (hotkey, i) in parsed {
            self.manager.register(hotkey)?;
            self.registered.push(hotkey);
            map.insert(hotkey.id(), i);
        }
        Ok(map)
    }
}

/// Shared validation + registration-set builder. No side effects — the settings
/// UI calls this on drafts, and it is pass 1 of `register_all`. Rules:
/// - empty hotkey = an unbound snippet: always fine, never registered
/// - non-empty hotkeys must parse, active or not (garbage never reaches disk)
/// - duplicates are only rejected among ACTIVE snippets (inactive ones may
///   deliberately share a combo — alternate texts you toggle between)
/// - the returned set is what actually registers: active snippets with valid keys
pub fn parse_all(config: &Config) -> Result<Vec<(HotKey, usize)>, AppError> {
    let mut parsed = Vec::new();
    let mut seen = HashSet::new();
    for (i, snippet) in config.snippets.iter().enumerate() {
        if snippet.hotkey.trim().is_empty() {
            continue; // unbound
        }
        let hotkey: HotKey = snippet.hotkey.parse().map_err(|_| {
            AppError::Config(format!("bad hotkey '{}' on snippet {}", snippet.hotkey, i + 1))
        })?;
        if !snippet.active {
            continue; // parked: validated but not registered, exempt from dup check
        }
        if !seen.insert(hotkey.id()) {
            return Err(AppError::Config(format!(
                "duplicate active hotkey '{}'",
                snippet.hotkey
            )));
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
                    text: format!("text{i}"),
                    hotkey: (*hk).into(),
                    active: true,
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
        assert!(msg.contains("snippet 1"), "error names the row: {msg}");
    }

    #[test]
    fn parse_all_rejects_duplicates() {
        let config = config_with(&["ctrl+alt+Digit1", "CTRL+ALT+Digit1"]); // case-insensitive dup
        let err = parse_all(&config).unwrap_err();
        assert!(err.to_string().contains("duplicate"), "{err}");
    }

    #[test]
    fn empty_hotkeys_are_unbound_not_errors() {
        let config = config_with(&["", "ctrl+alt+Digit1", "   "]);
        let parsed = parse_all(&config).expect("empties must be skipped, not rejected");
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].1, 1); // original index preserved
    }

    #[test]
    fn inactive_snippets_validate_but_do_not_register() {
        let mut config = config_with(&["ctrl+alt+Digit1", "ctrl+alt+Digit2"]);
        config.snippets[1].active = false;
        let parsed = parse_all(&config).expect("inactive valid snippet is fine");
        assert_eq!(parsed.len(), 1);

        // garbage on an INACTIVE snippet still errors — never reaches disk
        config.snippets[1].hotkey = "ctrl+alt+Banana".into();
        assert!(parse_all(&config).is_err());
    }

    #[test]
    fn inactive_duplicate_of_active_combo_is_allowed() {
        let mut config = config_with(&["ctrl+alt+Digit1", "ctrl+alt+Digit1"]);
        config.snippets[1].active = false;
        let parsed = parse_all(&config).expect("parked twin of an active combo is legal");
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].1, 0);
    }
}
