use std::fs;
use std::path::{Path, PathBuf};

use draconic_diagnostics::{Diagnostic, Span};
use draconic_pkg::{
    ensure_within_package, find_package_checkout_root, looks_like_module_path_import,
    resolve_module_import,
};

use crate::ACTIVE_PACKAGES;

pub(crate) fn lexical_normalize_path(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for comp in path.components() {
        match comp {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                out.pop();
            }
            other => out.push(other),
        }
    }
    out
}

pub(crate) fn normalize_path(path: &Path) -> Result<PathBuf, Diagnostic> {
    if path.exists() {
        fs::canonicalize(path).map_err(|e| {
            Diagnostic::new(
                format!("canonicalize {}: {e}", path.display()),
                Span::dummy(),
            )
        })
    } else {
        Ok(path.to_path_buf())
    }
}

pub(crate) fn resolve_specifier(
    parent: &Path,
    spec: &str,
    span: Span,
) -> Result<PathBuf, Diagnostic> {
    if spec.starts_with("./") || spec.starts_with("../") {
        let joined = parent.join(spec);
        if joined.exists() {
            let resolved = fs::canonicalize(&joined).map_err(|e| {
                Diagnostic::new(format!("canonicalize {}: {e}", joined.display()), span)
            })?;
            // K06.02: if the importer lives in a package checkout, stay inside it.
            if let Some(package_root) = find_package_checkout_root(parent) {
                ensure_within_package(&resolved, &package_root, spec)
                    .map_err(|e| Diagnostic::new(e.to_string(), span))?;
            }
            return Ok(resolved);
        }
        return Err(Diagnostic::new(
            format!("cannot resolve module `{spec}` from {}", parent.display()),
            span,
        ));
    }

    // K06.01: module-path imports via lock + module cache.
    if looks_like_module_path_import(spec) {
        let packages = ACTIVE_PACKAGES.with(|slot| slot.borrow().clone());
        if let Some(ctx) = packages {
            return resolve_module_import(spec, &ctx.lock, &ctx.cache)
                .map(|r| r.file)
                .map_err(|e| Diagnostic::new(e.to_string(), span));
        }
        return Err(Diagnostic::new(
            format!(
                "module-path import `{spec}` requires draconic.lock and module cache (no package context)"
            ),
            span,
        ));
    }

    Err(Diagnostic::new(
        format!("only relative module specifiers are supported (got `{spec}`)"),
        span,
    ))
}
