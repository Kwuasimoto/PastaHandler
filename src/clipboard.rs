use crate::error::AppError;


pub struct ClipboardManager {
    clipboard: arboard::Clipboard
}

impl ClipboardManager {
    pub fn new(clipboard: arboard::Clipboard) -> Self {
        Self { clipboard }
    }

    pub fn set_clipboard_text(&mut self, text: &str) -> Result<(), AppError> {
        let mut last_err = None;
        for _ in 0..3 {
            match self.clipboard.set_text(text) {
                Ok(()) => return Ok(()),
                Err(e) => {
                    last_err = Some(e);
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
            }
        }
        Err(AppError::Clipboard(last_err.unwrap())) 
    }
}