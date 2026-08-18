//! Minimal hand-rolled Win32 FFI shared by both processes. Narrow by design —
//! see COMPLIANCE.md: top-level window titles, ordinary window messages, a
//! named mutex for single-instancing, and DWM cosmetics on our own window
//! (focus border, blur-behind accent). No process handles are opened, nothing
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
    fn GetModuleHandleW(name: *const u16) -> isize;
    fn GetProcAddress(module: isize, name: *const core::ffi::c_char)
        -> *const core::ffi::c_void;
}

#[link(name = "dwmapi")]
unsafe extern "system" {
    fn DwmSetWindowAttribute(
        hwnd: isize,
        attr: u32,
        value: *const core::ffi::c_void,
        size: u32,
    ) -> i32;
}

#[repr(C)]
struct AccentPolicy {
    accent_state: u32,
    flags: u32,
    gradient_color: u32,
    animation_id: u32,
}

#[repr(C)]
struct WindowCompositionAttribData {
    attrib: u32,
    data: *mut core::ffi::c_void,
    size: usize,
}

const WCA_ACCENT_POLICY: u32 = 19;
const ACCENT_DISABLED: u32 = 0;
const ACCENT_ENABLE_BLURBEHIND: u32 = 3;

type SetWindowCompositionAttributeFn =
    unsafe extern "system" fn(isize, *mut WindowCompositionAttribData) -> i32;

/// The switch that makes per-pixel transparency REAL: the BLURBEHIND accent
/// policy has DWM composite whatever is behind the window — blurred, frosted-
/// glass style — wherever the canvas leaves alpha open. Without it,
/// framebuffer alpha renders on black. Undocumented but decade-stable
/// (Windows Terminal, PowerToys, Tauri vibrancy all ride it) — and being
/// undocumented it is absent from the SDK import lib, hence the
/// GetProcAddress lookup. Choices that matter: BLURBEHIND (3) composites
/// windows behind, while TRANSPARENTGRADIENT (2) shows only the bare
/// wallpaper on current builds; ACRYLICBLURBEHIND (4) has the known
/// window-drag lag. Requires the glow renderer — wgpu presents with
/// alpha-mode IGNORE, so its framebuffer alpha never reaches this layer.
pub fn set_blur(hwnd: isize, on: bool) {
    let user32: Vec<u16> = "user32.dll\0".encode_utf16().collect();
    let func: Option<SetWindowCompositionAttributeFn> = unsafe {
        let module = GetModuleHandleW(user32.as_ptr());
        std::mem::transmute(GetProcAddress(module, c"SetWindowCompositionAttribute".as_ptr()))
    };
    let Some(set_attribute) = func else {
        crate::logging::log_event("glass accent unavailable (no SetWindowCompositionAttribute)");
        return;
    };
    let mut policy = AccentPolicy {
        // off = ACCENT_DISABLED: the desktop still composites through the
        // canvas alpha, just SHARP instead of blurred (measured: disabling
        // the accent leaves DWM honoring the glow framebuffer alpha)
        accent_state: if on { ACCENT_ENABLE_BLURBEHIND } else { ACCENT_DISABLED },
        flags: 0,
        gradient_color: 0x0100_0000, // ABGR, near-invisible tint; canvas owns the look
        animation_id: 0,
    };
    let mut data = WindowCompositionAttribData {
        attrib: WCA_ACCENT_POLICY,
        data: (&mut policy as *mut AccentPolicy).cast(),
        size: size_of::<AccentPolicy>(),
    };
    let ok = unsafe { set_attribute(hwnd, &mut data) };
    crate::logging::log_event(&format!(
        "blur {} hwnd={hwnd} ok={ok}",
        if on { "on" } else { "off" }
    ));
}

const DWMWA_BORDER_COLOR: u32 = 34;
const DWMWA_COLOR_DEFAULT: u32 = 0xFFFF_FFFF;
const DWMWA_COLOR_NONE: u32 = 0xFFFF_FFFE;

/// Show or hide Windows 11's accent-colored focus border on the given window
/// (the "focus outline"). Windows 10 ignores the attribute — harmless.
///
/// Takes the HWND directly (eframe's Frame provides it). NEVER find it via
/// the enumeration helpers from inside a frame: GetWindowTextW on a window of
/// the calling process sends WM_GETTEXT into the window procedure, and winit
/// mid-frame deadlocks on that re-entrancy — the white "Not Responding" hang.
pub fn set_system_border(hwnd: isize, show: bool) {
    let color: u32 = if show { DWMWA_COLOR_DEFAULT } else { DWMWA_COLOR_NONE };
    let hr = unsafe {
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_BORDER_COLOR,
            &color as *const u32 as *const core::ffi::c_void,
            size_of::<u32>() as u32,
        )
    };
    // fires once per toggle, not per frame — cheap, and DWM failures are
    // otherwise invisible (the attribute cannot be read back)
    crate::logging::log_event(&format!("focus outline {} hwnd={hwnd} hr={hr:#x}", if show { "on" } else { "off" }));
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
