use std::time::{Duration, Instant};

use arboard::Clipboard;
use global_hotkey::{GlobalHotKeyEvent, HotKeyState};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tray_icon::menu::MenuEvent;

use crate::{
    clipboard::ClipboardManager, config::ConfigFile, error::AppError, hotkeys::Hotkeys, tray::Tray,
};

pub fn spawn_settings() {
    match std::env::current_exe() {
        Ok(exe) => {
            if let Err(e) = std::process::Command::new(exe).arg("--settings").spawn() {
                eprintln!("failed to launch settings: {e}");
            }
        }
        Err(e) => eprintln!("current_exe failed: {e}"),
    }
}

pub fn run(config_file: ConfigFile) -> Result<(), AppError> {
    let mut config = config_file.read()?;
    let mut clipboard_manager = ClipboardManager::new(Clipboard::new()?);
    let mut hotkeys = Hotkeys::new()?;
    let mut map = hotkeys.register_all(&config)?;
    let mut last_mtime = config_file.mtime();

    let event_loop = EventLoopBuilder::new().build();
    let hotkey_rx = GlobalHotKeyEvent::receiver();
    let menu_rx = MenuEvent::receiver();
    let tray = Tray::new()?;

    if config.open_settings_on_launch {
        spawn_settings();
    }

    event_loop.run(move |_event, _target, control_flow| {
        *control_flow = ControlFlow::WaitUntil(Instant::now() + Duration::from_secs(1));

        // heartbeat: a stat per wake is free; real work only when the file changed
        let mtime = config_file.mtime();
        if mtime != last_mtime {
            last_mtime = mtime;
            match config_file.read() {
                Ok(new) if new == config => {} // touched but unchanged: skip re-register
                Ok(new) => match hotkeys.register_all(&new) {
                    Ok(new_map) => {
                        // swap as a pair — map and config must never disagree
                        map = new_map;
                        config = new;
                    }
                    Err(e) => eprintln!("hotkey re-register failed: {e}"),
                },
                Err(e) => eprintln!("config reload failed: {e}"),
            }
        }

        // drain, don't sip: multiple events can queue between wakes
        while let Ok(key_event) = hotkey_rx.try_recv() {
            if key_event.state == HotKeyState::Pressed
                && let Some(&idx) = map.get(&key_event.id)
                && let Err(e) = clipboard_manager.set_clipboard_text(&config.snippets[idx].text)
            {
                eprintln!("{e}");
            }
        }

        while let Ok(menu_event) = menu_rx.try_recv() {
            if menu_event.id == tray.quit_id {
                *control_flow = ControlFlow::Exit;
            } else if menu_event.id == tray.open_id {
                spawn_settings();
            }
        }
    })
}
