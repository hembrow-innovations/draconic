//! Package manager support: `draconic.toml` manifests and related types (Roadmap K).
//!
//! K01: combined manifest surface — module path, deps, optional path→git URL map.
//! K01.01: parse own module path + dependencies map (path → version req).
//! K01.02: write/round-trip `draconic.toml` with stable dependency order.
//! K01.03: schema validation (module paths, version reqs, unknown fields) + diagnostics.
//! K01.04: optional URL map (path → git URL); default derive `https://{module_path}.git`.
//! K02: lockfile (`draconic.lock`) resolved pins — path, version, git URL, commit OID, tree SHA-256.
//! K02.01: lock entry — path + version + git URL + commit OID + content hash SHA-256.
//! K02.02: parse/write `draconic.lock`; reject malformed.
//! K02.03: stable lock serialize — sorted paths; byte-identical rewrite when unchanged.
//! K03.01: module cache layout keyed by module path + commit OID.
//! K03.02: git clone/fetch into cache VCS store (HTTPS; fixture repos in tests).
//! K03.03: checkout pinned OID into mod store; cache hit skips network.
//! K03.04: content hash SHA-256 over canonical package tree.
//! K04: version resolve — semver tag → commit OID; fail closed; direct-deps → lock pins.
//! K04.01: resolve version req against git tags; highest matching semver.
//! K04.02: fail closed: no match / non-semver-only / empty → diagnostic.
//! K04.03: resolve direct-deps set → lock pins (v1: direct only).
//! K05: CLI `draconic get` / `draconic mod tidy` — one get/tidy package surface.
//! K05.01: `draconic get <module_path>@<ver>` — fetch, update manifest+lock+cache.
//! K05.02: `draconic mod tidy` — lock matches manifest; fetch missing; prune unused.
//! K06.01: resolve module-path imports (`github.com/org/pkg` + subpath) → cache file.
//! K06.02: package boundary — reject path escape outside package checkout root.
//! K06.03: coexist with E11 relative imports (see linker + `tests/packages`).
//! K07: build integration — auto-fetch missing locked cache; `--offline`; lock pins win.
//! K07.01: ensure locked cache entries (auto-fetch missing pins for build).
//! K07.02: offline ensure — cache only; miss → fixit (no network).
//! K07.03: build prefers lock pins; does not float versions when lock present.
//! K08: integrity — verify lock hashes; refuse tampered cache (K08.01 + K08.02).
//! K08.01: recompute tree SHA-256; match lock `content_hash` or hard-fail.
//! K08.02: refuse mismatched checkout OID vs lock pin; no silent wrong tree.
//! K11: post-v1 packaging (not v1 bar) — later children are opt-in, never silent v1.
//! K11.01: private git auth — HTTPS token or SSH; fail closed; never persist secrets.
//! K11.02: `replace` directive — fork git source or local path override.
//! K11.03: multi-module monorepo — module path may map to a git subdirectory.
//! K11.04: module proxy/mirror (GOPROXY-shaped); git identity stays canonical.
//! K11.05: yank/retract when an advisory source is configured; else not a v1 check.
//! D02.01: optional/required toolchain version pin in `draconic.toml`.
//! D02.02: CLI compares running toolchain version to that pin (warn or hard-fail).

mod auth;
mod cache;
mod ensure;
mod get;
mod hash;
mod import_resolve;
mod later;
mod lock;
mod manifest;
mod parse;
mod proxy;
mod replace;
mod resolve;
mod subdir;
mod tidy;
mod toolchain;
mod validate;
mod write;
mod yank;

pub use auth::{
    clone_url_with_auth, git_auth_from_vars, git_auth_rejected, git_ssh_command,
    is_git_auth_failure, is_https_git_url, is_ssh_git_url, redact_secrets, sanitize_stored_git_url,
    GitAuth, GitAuthError,
};
pub use cache::{
    entry_rel_path, is_entry_under_root, vcs_rel_path, CacheFetchError, CachePathError, ModuleCache,
};
pub use ensure::{
    ensure_locked_entries, ensure_locked_for_entry, EnsureLockedError, EnsureLockedResult,
};
pub use get::{
    default_cache_root, get_package, get_package_spec, get_package_with_auth, parse_get_spec,
    GetError, GetResult, DEFAULT_CACHE_DIR_NAME, LOCK_FILE, MANIFEST_FILE,
};
pub use hash::{
    content_hash_tree, read_checkout_oid, verify_content_hash, verify_package_integrity,
    ContentHashError, ContentHashVerifyError, PackageIntegrityError,
};
pub use import_resolve::{
    ensure_within_package, find_package_checkout_root, looks_like_module_path_import,
    match_locked_package, path_is_within_root, resolve_module_import, ImportResolveError,
    ResolvedImport,
};
pub use later::{LaterPackaging, LaterPackagingError};
pub use lock::{parse_lock, write_lock, LockEntry, LockEntryError, LockFile, LockFileError};
pub use manifest::{Manifest, ManifestError, ToolchainPin};
pub use parse::parse_manifest;
pub use proxy::{
    mirror_fetch_url, module_proxy_from_vars, ModuleProxy, ProxyEntry, ProxyError, ProxyFetch,
    PROXY_ENV,
};
pub use replace::ReplaceSource;
pub use resolve::{
    resolve_direct_deps, resolve_direct_deps_with_advisory, resolve_highest_matching_tag,
    ResolveDirectError, ResolveError, ResolvedVersion,
};
pub use subdir::{derive_package_subdir, repo_path_from_git_url, validate_package_subdir};
pub use tidy::{mod_tidy, mod_tidy_default_cache, TidyError, TidyResult};
pub use toolchain::{check_toolchain_pin, check_toolchain_pin_for_entry, ToolchainPinStatus};
pub use validate::validate_manifest;
pub use write::write_manifest;
pub use yank::{advisory_from_vars, AdvisoryError, AdvisorySource, YankKind, ADVISORY_ENV};

pub(crate) use validate::{validate_git_url, validate_module_path, validate_version_req};

/// Default git URL for a module path: `https://{module_path}.git` (K01.04 / ADR-0009).
pub fn default_git_url(module_path: &str) -> String {
    format!("https://{module_path}.git")
}

/// Resolve the git URL for `module_path`.
///
/// Order: `[replace]` (K11.02) wins over `[urls]` (K01.04), else [`default_git_url`].
pub fn resolve_git_url(manifest: &Manifest, module_path: &str) -> String {
    if let Some(repl) = manifest.replace.get(module_path) {
        return repl.fetch_url();
    }
    manifest
        .urls
        .get(module_path)
        .cloned()
        .unwrap_or_else(|| default_git_url(module_path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn manifest(module: &str, deps: &[(&str, &str)]) -> Manifest {
        manifest_with_urls(module, deps, &[])
    }

    fn manifest_with_urls(module: &str, deps: &[(&str, &str)], urls: &[(&str, &str)]) -> Manifest {
        Manifest {
            module: module.to_string(),
            dependencies: deps
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            urls: urls
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            replace: BTreeMap::new(),
            toolchain: None,
        }
    }

    #[test]
    fn default_git_url_derives_https_module_path_git() {
        assert_eq!(
            default_git_url("github.com/org/pkg"),
            "https://github.com/org/pkg.git"
        );
        assert_eq!(
            default_git_url("gitlab.com/group/sub/mod"),
            "https://gitlab.com/group/sub/mod.git"
        );
    }

    #[test]
    fn resolve_git_url_uses_default_when_urls_empty() {
        let m = manifest("github.com/acme/app", &[("github.com/org/lib", "1.0.0")]);
        assert_eq!(
            resolve_git_url(&m, "github.com/org/lib"),
            "https://github.com/org/lib.git"
        );
        assert_eq!(
            resolve_git_url(&m, "github.com/other/util"),
            "https://github.com/other/util.git"
        );
    }

    #[test]
    fn resolve_git_url_prefers_urls_map_override() {
        let m = manifest_with_urls(
            "github.com/acme/app",
            &[("github.com/org/lib", "1.0.0")],
            &[(
                "github.com/org/lib",
                "https://git.example.com/mirror/lib.git",
            )],
        );
        assert_eq!(
            resolve_git_url(&m, "github.com/org/lib"),
            "https://git.example.com/mirror/lib.git"
        );
        assert_eq!(
            resolve_git_url(&m, "github.com/other/util"),
            "https://github.com/other/util.git"
        );
    }
}
