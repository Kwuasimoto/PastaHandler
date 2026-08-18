//! Minimal hand-rolled Win32 FFI shared by both processes. Narrow by design —
//! see COMPLIANCE.md: top-level window titles, ordinary window messages, and a
//! named mutex for single-instancing. No process handles are opened, nothing
//! is injected or simulated; the compliance grep stays empty.

/// The settings window's title — the single source of truth. The window is
/// FOUND by this exact string, so the eframe app name, the header label, and
/// the matcher below all read it from here; they can never drift apart.
pub const SETTINGS_WINDOW_TITLE: &str = "Pasta Handler Settings";

const WM_CLOSE: u32 = 0x0010;
const SW_RESTORE: i32 = 9;
const ERROR_ALREADY_EXISTS: i32 = 183;

#[link(name = "user32")]
unsafe extern "system" {
    fn EnumWindows(
        callback: unsafe extern "system" fn(isize, isize) -> i32,
        lparam: isize,
    ) -> i32;
    fn GetWindowTextW(hwnd: isize, buf: *mut u16, max_count: i32) -> i32;
    fn PostMessageW(hwnd: isize, msg: u32, wparam: usize, lparam: isize) -> i32;
    fn SetForegroundWindow(hwnd: isize) -> i32;
    fn ShowWindow(hwnd: isize, cmd: i32) -> i32;
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn CreateMutexW(
        attrs: *mut core::ffi::c_void,
        initial_owner: i32,
        name: *const u16,
    ) -> isize;
}

unsafe extern "system" fn collect_settings_windows(hwnd: isize, lparam: isize) -> i32 {
    let mut buf = [0u16; 64];
    let len = unsafe { GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32) };
    if String::from_utf16_lossy(&buf[..len.max(0) as usize]) == SETTINGS_WINDOW_TITLE {
        unsafe { (*(lparam as *mut Vec<isize>)).push(hwnd) };
    }
    1 // keep enumerating — several windows may exist
}

fn settings_windows() -> Vec<isize> {
    let mut found: Vec<isize> = Vec::new();
    unsafe { EnumWindows(collect_settings_windows, &mut found as *mut _ as isize) };
    found
}

/// Ask every settings window to close — the same WM_CLOSE the ✕ sends, so
/// eframe shuts down cleanly. Used by tray Quit: quit means the whole app.
pub fn close_settings_windows() {
    for hwnd in settings_windows() {
        unsafe { PostMessageW(hwnd, WM_CLOSE, 0, 0) };
    }
}

/// Bring the (first) existing settings window to the front, un-minimizing it
/// if needed. Used by the second settings instance instead of opening a twin.
pub fn focus_settings_window() {
    if let Some(&hwnd) = settings_windows().first() {
        unsafe {
            ShowWindow(hwnd, SW_RESTORE);
            SetForegroundWindow(hwnd);
        }
    }
}

/// Claim a per-user named mutex; false means another instance already holds
/// it. The handle is deliberately leaked — the claim must live exactly as
/// long as the process, and process exit is what releases it.
pub fn claim_single_instance(name: &str) -> bool {
    let wide: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
    let handle = unsafe { CreateMutexW(std::ptr::null_mut(), 0, wide.as_ptr()) };
    handle != 0 && std::io::Error::last_os_error().raw_os_error() != Some(ERROR_ALREADY_EXISTS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn second_claim_on_the_same_name_fails() {
        // unique per test run so parallel test invocations can't collide
        let name = format!("Local\\pastahandler-test-{}", std::process::id());
        assert!(claim_single_instance(&name), "first claim wins");
        assert!(!claim_single_instance(&name), "second claim must lose");
    }
}
