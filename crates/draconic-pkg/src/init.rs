//! `draconic mod init <module_path>`: write a first module manifest.

use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use crate::get::MANIFEST_FILE;
use crate::{validate_manifest, validate_module_path, write_manifest, Manifest, ManifestError};

/// Result of a successful `mod init`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InitResult {
    /// Module path written into the manifest.
    pub module: String,
    /// Path of the written `draconic.toml`.
    pub manifest_path: PathBuf,
}

/// Error while writing a first module manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InitError {
    /// Module path schema failed.
    InvalidPath { path: String, reason: &'static str },
    /// Workspace already has `draconic.toml`.
    AlreadyExists { path: String },
    /// Manifest validate failed after path check.
    Manifest(String),
    /// Creating or writing the manifest failed.
    Io(String),
}

impl fmt::Display for InitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InitError::InvalidPath { path, reason } => {
                write!(f, "mod init: invalid module path `{path}`: {reason}")
            }
            InitError::AlreadyExists { path } => {
                write!(f, "mod init: `{path}` already exists")
            }
            InitError::Manifest(msg) => write!(f, "mod init: {msg}"),
            InitError::Io(msg) => write!(f, "mod init: {msg}"),
        }
    }
}

impl std::error::Error for InitError {}

impl From<ManifestError> for InitError {
    fn from(e: ManifestError) -> Self {
        match e {
            ManifestError::InvalidModulePath { path, reason } => {
                InitError::InvalidPath { path, reason }
            }
            other => InitError::Manifest(other.to_string()),
        }
    }
}

/// Write `draconic.toml` with `module = "<module_path>"` using the existing schema.
///
/// Refuses to overwrite an existing manifest. Does not write `version` or `exports`.
pub fn mod_init(workspace: &Path, module_path: &str) -> Result<InitResult, InitError> {
    if let Err(reason) = validate_module_path(module_path) {
        return Err(InitError::InvalidPath {
            path: module_path.to_string(),
            reason,
        });
    }
    let manifest = Manifest {
        module: module_path.to_string(),
        dependencies: BTreeMap::new(),
        urls: BTreeMap::new(),
        replace: BTreeMap::new(),
        toolchain: None,
    };
    validate_manifest(&manifest)?;
    let manifest_path = workspace.join(MANIFEST_FILE);
    let body = write_manifest(&manifest);
    let mut file = match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&manifest_path)
    {
        Ok(f) => f,
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
            return Err(InitError::AlreadyExists {
                path: manifest_path.display().to_string(),
            });
        }
        Err(e) => return Err(InitError::Io(e.to_string())),
    };
    file.write_all(body.as_bytes())
        .map_err(|e| InitError::Io(e.to_string()))?;
    Ok(InitResult {
        module: module_path.to_string(),
        manifest_path,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(tag: &str) -> PathBuf {
        static N: AtomicU64 = AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "draconic-pkg-init-{tag}-{}-{}-{}",
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
    fn writes_module_only() {
        let ws = temp_dir("write");
        let r = mod_init(&ws, "github.com/org/pkg").unwrap();
        assert_eq!(r.module, "github.com/org/pkg");
        let src = fs::read_to_string(&r.manifest_path).unwrap();
        assert_eq!(src, "module = \"github.com/org/pkg\"\n");
        assert!(!src.contains("version"));
        assert!(!src.contains("exports"));
        let _ = fs::remove_dir_all(&ws);
    }

    #[test]
    fn refuses_existing_manifest() {
        let ws = temp_dir("exists");
        let path = ws.join(MANIFEST_FILE);
        fs::write(&path, "module = \"github.com/org/old\"\n").unwrap();
        let err = mod_init(&ws, "github.com/org/new").unwrap_err();
        assert!(matches!(err, InitError::AlreadyExists { .. }), "{err:?}");
        let src = fs::read_to_string(&path).unwrap();
        assert_eq!(src, "module = \"github.com/org/old\"\n");
        let _ = fs::remove_dir_all(&ws);
    }

    #[test]
    fn rejects_invalid_module_path() {
        let ws = temp_dir("bad");
        let err = mod_init(&ws, "notadomain").unwrap_err();
        assert!(matches!(err, InitError::InvalidPath { .. }), "{err:?}");
        assert!(!ws.join(MANIFEST_FILE).exists());
        let _ = fs::remove_dir_all(&ws);
    }
}
