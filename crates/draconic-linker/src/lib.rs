//! Link ESM modules into a single Program (ROADMAP E11.01–E11.04, E18.29–E18.31).
//!
//! Loads an entry file, follows static relative `import … from "…"`, mangles
//! per-module top-level bindings to avoid collisions, rewrites import locals to
//! the exporter's mangled names, and concatenates dependency bodies before the
//! entry. Supports named, default, namespace (`import * as ns`), `export * from`,
//! `export * as ns from`, and `export { … } from` re-exports, including cyclic
//! graphs (live bindings via shared cells).
//!
//! K06.01: non-relative module-path imports (`github.com/org/pkg` + subpath)
//! resolve via `draconic.lock` + module cache when a package context is present
//! (explicit or discovered from an ancestor workspace).
//!
//! K06.02: relative imports from inside a package checkout must not resolve
//! outside that package root (package boundary).
//!
//! K06.03: E11 static relative imports and module-path imports coexist in one
//! graph (consumer locals + packages; package-internal relatives).

use std::cell::RefCell;
use std::fs;
use std::path::Path;
#[cfg(test)]
use std::path::PathBuf;
use std::sync::Arc;

use draconic_ast::Program;
use draconic_diagnostics::Diagnostic;
use draconic_pkg::{default_cache_root, parse_lock, ModuleCache, LOCK_FILE};

mod dynamic;
mod dynamic_collect;
mod eval;
mod export;
mod json;
mod link;
mod load;
mod namespace;
mod path;
mod rename;
mod spans;

use crate::load::Loader;
use crate::path::normalize_path;

thread_local! {
    /// Active package context while loading a graph (K06.01).
    pub(crate) static ACTIVE_PACKAGES: RefCell<Option<Arc<PackageLinkContext>>> = const { RefCell::new(None) };
}

/// Package resolution context for module-path imports (K06.01).
#[derive(Debug, Clone)]
pub struct PackageLinkContext {
    pub lock: draconic_pkg::LockFile,
    pub cache: ModuleCache,
}

/// Parse `entry` and all static imports into one linked Program.
///
/// Discovers `draconic.lock` + default module cache by walking ancestors of
/// `entry` when present; otherwise only relative specifiers are accepted.
pub fn link_entry(entry: &Path) -> Result<Program, Diagnostic> {
    Ok(link_entry_with_named_exports(entry)?.0)
}

/// Link `entry` and keep the entry named-export table (public → local after flatten).
pub fn link_entry_with_named_exports(
    entry: &Path,
) -> Result<(Program, Vec<(String, String)>), Diagnostic> {
    let pkgs = discover_package_context(entry);
    link_loaded(entry, pkgs.as_ref())
}

/// Link with an explicit lock + cache for module-path imports (K06.01).
pub fn link_entry_with_packages(
    entry: &Path,
    packages: Option<&PackageLinkContext>,
) -> Result<Program, Diagnostic> {
    Ok(link_loaded(entry, packages)?.0)
}

fn link_loaded(
    entry: &Path,
    packages: Option<&PackageLinkContext>,
) -> Result<(Program, Vec<(String, String)>), Diagnostic> {
    let entry = normalize_path(entry)?;
    let mut loader = Loader::new(packages.map(|p| Arc::new(p.clone())));
    loader.load_graph(&entry)?;
    loader.link(&entry)
}

/// Walk parents of `entry` for `draconic.lock`; use default cache under that workspace.
fn discover_package_context(entry: &Path) -> Option<PackageLinkContext> {
    let start = if entry.is_file() {
        entry.parent()?
    } else {
        entry
    };
    let mut dir = start;
    loop {
        let lock_path = dir.join(LOCK_FILE);
        if lock_path.is_file() {
            let src = fs::read_to_string(&lock_path).ok()?;
            let lock = parse_lock(&src).ok()?;
            let cache = ModuleCache::new(default_cache_root(dir));
            return Some(PackageLinkContext { lock, cache });
        }
        dir = dir.parent()?;
    }
}

#[cfg(test)]
pub(crate) fn temp_link_dir(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "draconic-link-{label}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&dir).unwrap();
    dir
}
