//! D02.02: CLI enforce/warn when the running toolchain ≠ `draconic.toml` pin.

use std::path::Path;
use std::process::ExitCode;

use draconic_pkg::{check_toolchain_pin_for_entry, ToolchainPinStatus};

/// Check the nearest `draconic.toml` pin for `start`.
///
/// Optional mismatch prints a warning and continues. Required mismatch
/// prints an error and returns `ExitCode(1)`. Version/help skip this.
pub(crate) fn enforce(start: &Path) -> Result<(), ExitCode> {
    let running = env!("CARGO_PKG_VERSION");
    match check_toolchain_pin_for_entry(start, running) {
        ToolchainPinStatus::Warn { pin, running } => {
            eprintln!("warning: toolchain pin is {pin} but this is {running}");
            Ok(())
        }
        ToolchainPinStatus::Mismatch { pin, running } => {
            eprintln!("error: toolchain pin requires {pin} but this is {running}");
            Err(ExitCode::from(1))
        }
        ToolchainPinStatus::Unpinned | ToolchainPinStatus::Match { .. } => Ok(()),
    }
}
