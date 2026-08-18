//! Best-effort event log, appended next to the config file
//! (`%APPDATA%\pastahandler\pastahandler.log`). The resident process has no
//! console once `windows_subsystem = "windows"` is set — this file is the only
//! place its failures remain visible.

use std::io::Write;

/// Print to stderr (visible in dev console runs) AND append to the log file.
pub fn warn(msg: &str) {
    eprintln!("{msg}");
    log_event(msg);
}

/// Append one timestamped line. Logging must never take the app down, so every
/// failure in here is ignored on purpose — the one sanctioned log-and-ignore
/// in this codebase.
pub fn log_event(msg: &str) {
    let Ok(path) = crate::config::Config::build_path() else { return };
    let Some(dir) = path.parent() else { return };
    let _ = std::fs::create_dir_all(dir);
    let log_path = dir.join("pastahandler.log");
    if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(log_path) {
        let _ = writeln!(file, "[{}] {}", timestamp_utc(), msg);
    }
}

fn timestamp_utc() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format_epoch(secs)
}

/// Epoch seconds -> "YYYY-MM-DD HH:MM:SSZ" without a date crate: the standard
/// civil-from-days algorithm (Howard Hinnant's), which the unit test pins.
fn format_epoch(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    let tod = secs % 86_400;
    let (h, mi, s) = (tod / 3600, (tod % 3600) / 60, tod % 60);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02} {h:02}:{mi:02}:{s:02}Z")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_epoch_matches_known_dates() {
        assert_eq!(format_epoch(0), "1970-01-01 00:00:00Z");
        assert_eq!(format_epoch(86_400), "1970-01-02 00:00:00Z");
        assert_eq!(format_epoch(951_782_400), "2000-02-29 00:00:00Z"); // leap day
        assert_eq!(format_epoch(1_755_468_245), "2025-08-17 22:04:05Z");
    }
}
