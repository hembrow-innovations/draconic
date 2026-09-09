use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::UNIX_EPOCH;

/// Poll `path` mtime and re-run `action` on change. Initial run is immediate.
/// Errors are printed; the loop continues. Exit with Ctrl-C (or kill in tests).
pub(crate) fn run_watch_loop(
    path: &Path,
    mut action: impl FnMut() -> Result<(), String>,
) -> ExitCode {
    let poll_ms = env::var("DRACONIC_WATCH_POLL_MS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(200)
        .max(10);

    eprintln!("watching {} (poll {poll_ms}ms)", path.display());

    let mut last_stamp = file_watch_stamp(path);
    if let Err(msg) = action() {
        eprintln!("error: {msg}");
    }

    loop {
        std::thread::sleep(std::time::Duration::from_millis(poll_ms));
        let stamp = file_watch_stamp(path);
        if stamp != last_stamp {
            last_stamp = stamp;
            if let Err(msg) = action() {
                eprintln!("error: {msg}");
            }
        }
    }
}

fn file_watch_stamp(path: &Path) -> Option<(u64, u64)> {
    let meta = fs::metadata(path).ok()?;
    let modified = meta.modified().ok()?.duration_since(UNIX_EPOCH).ok()?;
    let len = meta.len();
    Some((modified.as_secs(), modified.subsec_nanos() as u64 ^ len))
}

/// Test hook: when `DRACONIC_WATCH_MARKER` is set, write an incrementing counter
/// after each successful check (used by U10 integration tests).
pub(crate) fn touch_watch_marker() {
    let Some(path) = env::var_os("DRACONIC_WATCH_MARKER") else {
        return;
    };
    let path = PathBuf::from(path);
    let next = fs::read_to_string(&path)
        .ok()
        .and_then(|s| s.trim().parse::<u64>().ok())
        .unwrap_or(0)
        .saturating_add(1);
    let _ = fs::write(&path, format!("{next}\n"));
}
