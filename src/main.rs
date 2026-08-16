pub mod config;
pub mod error;
pub mod clipboard;
pub mod hotkeys;

use arboard::Clipboard;
// #![windows_subsystem = "windows"]
use global_hotkey::{GlobalHotKeyEvent, HotKeyState};
use tao::event_loop::{ControlFlow, EventLoopBuilder};

use crate::{clipboard::ClipboardManager, config::{Config, ConfigFile}, error::AppError, hotkeys::Hotkeys};

fn main() -> Result<(), AppError> {
    let config = ConfigFile::new(Config::get_path()?).read()?;
    let mut clipboard_manager = ClipboardManager::new(Clipboard::new()?);

    let hotkeys = Hotkeys::new()?;
    let map = hotkeys.register_all(&config)?;

    let event_loop = EventLoopBuilder::new().build();
    let hotkey_rx = GlobalHotKeyEvent::receiver();

    event_loop.run(move |_event, _target, control_flow| {
        *control_flow = ControlFlow::Poll;

        if let Ok(key_event) = hotkey_rx.try_recv() {
            if let Some(&idx) = map.get(&key_event.id) {
                if let Err(e) = clipboard_manager.set_clipboard_text(&config.snippets[idx].text) {
                    eprintln!("{e}");
                }
            }
        }
    })
}
