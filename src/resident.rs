//! The resident process: the tray-dwelling half of the app. Registers global
//! hotkeys, writes snippets to the clipboard, reloads the config live when the
//! settings process (or a hand edit) changes it. Deliberately GPU-free.

use std::time::{Duration, Instant};

use arboard::Clipboard;
use global_hotkey::{GlobalHotKeyEvent, HotKeyState};
use tao::event::Event;
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tray_icon::menu::MenuEvent;

use crate::{
    clipboard::ClipboardManager, config::ConfigFile, error::AppError, hotkeys::Hotkeys, tray::Tray,
};

/// Events forwarded into the tao loop by the global-hotkey / tray-icon handlers.
/// Sent via the loop's proxy, they WAKE the loop immediately — unlike the crates'
/// default channels, which only get drained when something else wakes us (that
/// was a paste-the-previous-snippet race: the 1s heartbeat lost to a fast Ctrl+V).
enum AppEvent {
    Hotkey(GlobalHotKeyEvent),
    Menu(MenuEvent),
}

pub fn spawn_settings() {
    match std::env::current_exe() {
        Ok(exe) => {
            if let Err(e) = std::process::Command::new(exe).arg("--settings").spawn() {
                crate::logging::warn(&format!("failed to launch settings: {e}"));
            }
        }
        Err(e) => crate::logging::warn(&format!("current_exe failed: {e}")),
    }
}

pub fn run(config_file: ConfigFile) -> Result<(), AppError> {
    // second resident = a double-click on the Start-menu entry while one is
    // already in the tray. Exit quietly; the first instance is the app.
    if !crate::win32::claim_single_instance("Local\\pastahandler-resident") {
        crate::logging::log_event("resident already running — duplicate exiting");
        return Ok(());
    }

    let mut config = config_file.read()?;
    let mut clipboard_manager = ClipboardManager::new(Clipboard::new()?);
    let mut hotkeys = Hotkeys::new()?;
    let mut map = hotkeys.register_all(&config)?;
    let mut last_mtime = config_file.mtime();

    let event_loop = EventLoopBuilder::<AppEvent>::with_user_event().build();

    // handlers run on the crates' dispatch path; the proxy wakes the loop NOW
    let proxy = event_loop.create_proxy();
    GlobalHotKeyEvent::set_event_handler(Some(move |e: GlobalHotKeyEvent| {
        let _ = proxy.send_event(AppEvent::Hotkey(e));
    }));
    let proxy = event_loop.create_proxy();
    MenuEvent::set_event_handler(Some(move |e: MenuEvent| {
        let _ = proxy.send_event(AppEvent::Menu(e));
    }));

    let tray = Tray::new()?;

    if config.open_settings_on_launch {
        spawn_settings();
    }

    event_loop.run(move |event, _target, control_flow| {
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
                    Err(e) => crate::logging::warn(&format!("hotkey re-register failed: {e}")),
                },
                Err(e) => crate::logging::warn(&format!("config reload failed: {e}")),
            }
        }

        match event {
            Event::UserEvent(AppEvent::Hotkey(key_event)) => {
                // `get` twice over: the map lookup filters unknown ids, and the
                // snippet lookup refuses to panic even if map and config were
                // ever to disagree (they're swapped as a pair, but an event
                // loop should never contain an indexing panic).
                if key_event.state == HotKeyState::Pressed
                    && let Some(&idx) = map.get(&key_event.id)
                    && let Some(snippet) = config.snippets.get(idx)
                    && let Err(e) = clipboard_manager.set_text(&snippet.text)
                {
                    crate::logging::warn(&format!("clipboard write failed: {e}"));
                }
            }
            Event::UserEvent(AppEvent::Menu(menu_event)) => {
                if menu_event.id == tray.quit_id {
                    crate::win32::close_settings_windows(); // quit means the whole app
                    *control_flow = ControlFlow::Exit;
                } else if menu_event.id == tray.open_id {
                    spawn_settings();
                }
            }
            _ => {}
        }
    })
}
