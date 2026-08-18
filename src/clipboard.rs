//! Clipboard writes with retry. The Windows clipboard is a single shared
//! resource: opening it can transiently fail while another process holds it
//! (clipboard managers, Office hooks), so writes retry briefly before failing.

use std::time::Duration;

use crate::error::AppError;

/// How many times to attempt a write before giving up.
const ATTEMPTS: u32 = 3;
/// Pause between attempts — long enough for another process to release the
/// clipboard, short enough that a hotkey press still feels instant.
const RETRY_DELAY: Duration = Duration::from_millis(50);

pub struct ClipboardManager {
    clipboard: arboard::Clipboard,
}

impl ClipboardManager {
    pub fn new(clipboard: arboard::Clipboard) -> Self {
        Self { clipboard }
    }

    /// Write `text` to the OS clipboard. `Ok` means Windows accepted the data —
    /// it owns the memory from then on, which is why the contents survive this
    /// process exiting and why no read-back verification is needed.
    pub fn set_text(&mut self, text: &str) -> Result<(), AppError> {
        let mut last_err = None;
        for _ in 0..ATTEMPTS {
            match self.clipboard.set_text(text) {
                Ok(()) => return Ok(()),
                Err(e) => {
                    last_err = Some(e);
                    std::thread::sleep(RETRY_DELAY);
                }
            }
        }
        // invariant, not a fallible unwrap: the loop ran at least once, so a
        // failure to return early means last_err is Some
        Err(AppError::Clipboard(last_err.expect("loop ran at least once")))
    }
}
