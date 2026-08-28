//! D02.02: compare the running toolchain version to a `draconic.toml` pin.

use std::fs;
use std::path::{Path, PathBuf};

use crate::{parse_manifest, Manifest, ToolchainPin, MANIFEST_FILE};

/// Outcome of comparing the running toolchain to a manifest pin (D02.02).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolchainPinStatus {
    /// No `draconic.toml`, unreadable/invalid manifest, or pin omitted.
    Unpinned,
    /// Running version equals the pin.
    Match { version: String },
    /// Optional pin differs — warn, do not fail.
    Warn { pin: String, running: String },
    /// Required pin differs — hard-fail.
    Mismatch { pin: String, running: String },
}

/// Compare `running` (CLI `CARGO_PKG_VERSION`) to an optional pin.
///
/// Equality is exact on the version string. `required = true` → [`ToolchainPinStatus::Mismatch`];
/// optional pin → [`ToolchainPinStatus::Warn`].
pub fn check_toolchain_pin(pin: Option<&ToolchainPin>, running: &str) -> ToolchainPinStatus {
    let Some(pin) = pin else {
        return ToolchainPinStatus::Unpinned;
    };
    if pin.version == running {
        return ToolchainPinStatus::Match {
            version: pin.version.clone(),
        };
    }
    if pin.required {
        ToolchainPinStatus::Mismatch {
            pin: pin.version.clone(),
            running: running.to_string(),
        }
    } else {
        ToolchainPinStatus::Warn {
            pin: pin.version.clone(),
            running: running.to_string(),
        }
    }
}

/// Walk ancestors of `start` for `draconic.toml` and check against `running`.
///
/// Invalid or unreadable manifests are treated as unpinned so Program-only
/// commands do not gain a new hard failure on a broken package file.
pub fn check_toolchain_pin_for_entry(start: &Path, running: &str) -> ToolchainPinStatus {
    let Some(manifest) = load_nearest_manifest(start) else {
        return ToolchainPinStatus::Unpinned;
    };
    check_toolchain_pin(manifest.toolchain.as_ref(), running)
}

fn load_nearest_manifest(start: &Path) -> Option<Manifest> {
    let mut dir: PathBuf = if start.is_file() {
        start.parent()?.to_path_buf()
    } else {
        start.to_path_buf()
    };
    loop {
        let path = dir.join(MANIFEST_FILE);
        if path.is_file() {
            let src = fs::read_to_string(&path).ok()?;
            return parse_manifest(&src).ok();
        }
        dir = dir.parent()?.to_path_buf();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir() -> PathBuf {
        static N: AtomicU64 = AtomicU64::new(0);
        let mut dir = std::env::temp_dir();
        dir.push(format!(
            "draconic-pkg-toolchain-{}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn omitted_pin_is_unpinned() {
        assert_eq!(
            check_toolchain_pin(None, "0.1.0"),
            ToolchainPinStatus::Unpinned
        );
    }

    #[test]
    fn matching_optional_pin_is_match() {
        let pin = ToolchainPin {
            version: "0.1.0".into(),
            required: false,
        };
        assert_eq!(
            check_toolchain_pin(Some(&pin), "0.1.0"),
            ToolchainPinStatus::Match {
                version: "0.1.0".into()
            }
        );
    }

    #[test]
    fn matching_required_pin_is_match() {
        let pin = ToolchainPin {
            version: "0.1.0".into(),
            required: true,
        };
        assert_eq!(
            check_toolchain_pin(Some(&pin), "0.1.0"),
            ToolchainPinStatus::Match {
                version: "0.1.0".into()
            }
        );
    }

    #[test]
    fn optional_mismatch_is_warn() {
        let pin = ToolchainPin {
            version: "9.9.9".into(),
            required: false,
        };
        assert_eq!(
            check_toolchain_pin(Some(&pin), "0.1.0"),
            ToolchainPinStatus::Warn {
                pin: "9.9.9".into(),
                running: "0.1.0".into(),
            }
        );
    }

    #[test]
    fn required_mismatch_is_mismatch() {
        let pin = ToolchainPin {
            version: "9.9.9".into(),
            required: true,
        };
        assert_eq!(
            check_toolchain_pin(Some(&pin), "0.1.0"),
            ToolchainPinStatus::Mismatch {
                pin: "9.9.9".into(),
                running: "0.1.0".into(),
            }
        );
    }

    #[test]
    fn entry_without_manifest_is_unpinned() {
        let dir = temp_dir();
        let src = dir.join("ok.drac");
        fs::write(&src, "let x = 1;\n").unwrap();
        assert_eq!(
            check_toolchain_pin_for_entry(&src, "0.1.0"),
            ToolchainPinStatus::Unpinned
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn entry_discovers_parent_required_mismatch() {
        let dir = temp_dir();
        fs::write(
            dir.join("draconic.toml"),
            "module = \"github.com/acme/app\"\ntoolchain = { version = \"9.9.9\", required = true }\n",
        )
        .unwrap();
        let nested = dir.join("src");
        fs::create_dir_all(&nested).unwrap();
        let src = nested.join("ok.drac");
        fs::write(&src, "let x = 1;\n").unwrap();
        assert_eq!(
            check_toolchain_pin_for_entry(&src, "0.1.0"),
            ToolchainPinStatus::Mismatch {
                pin: "9.9.9".into(),
                running: "0.1.0".into(),
            }
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn invalid_manifest_is_unpinned() {
        let dir = temp_dir();
        fs::write(dir.join("draconic.toml"), "not = [toml\n").unwrap();
        let src = dir.join("ok.drac");
        fs::write(&src, "let x = 1;\n").unwrap();
        assert_eq!(
            check_toolchain_pin_for_entry(&src, "0.1.0"),
            ToolchainPinStatus::Unpinned
        );
        let _ = fs::remove_dir_all(&dir);
    }
}
