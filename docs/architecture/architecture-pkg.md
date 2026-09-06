---
id: "architecture-pkg"
title: "Packages"
kind: architecture
description: "Go-style git-backed modules: draconic.toml, draconic.lock, cache, integrity, and opt-in later knobs."
domain: draconic
area: tooling
tags: []
created_at: "2026-09-06"
updated_at: "2026-09-06"
---

# Packages

## Overview

`draconic-pkg` implements Roadmap K as decided in [[0009-go-style-git-packages]]. Packages are git-backed with no v1 registry. Imports use a Go-like module path (`github.com/org/pkg`). The manifest may map path → git URL when default `https://{module_path}.git` is wrong. Versions are semver git tags. The lock pins commit OID and a SHA-256 of the package tree. [[architecture-cli]] exposes `get` and `mod tidy`; `build` auto-fetches missing lock pins unless `--offline`.

## Context

v1 rejected an npm-compatible registry, a crates.io clone, lockfile-optional floating builds, and replacing ESM with a different module syntax. Resolve still lands on ESM files inside a checkout; [[architecture-linker]] loads those files. End-to-end coverage lives under `tests/packages` (temp git upstream, consumer Program, compile + Node).

## Design

**Manifest** (`draconic.toml`): required `module` string; optional `[dependencies]` (path → version req), `[urls]`, `[replace]`, `toolchain`. Known top-level keys only. Module paths need a domain-like first segment and at least two `/` parts. Version reqs are semver-shaped (`^` `~` `>=` `<=` `>` `<` `=` , optional `v`, partial MAJOR[.MINOR[.PATCH]]). Write is stable (sorted keys, byte-identical rewrite). `resolve_git_url` prefers `[replace]`, then `[urls]`, else the default HTTPS URL.

**Lock** (`draconic.lock`): versioned document of `LockEntry` values — path, concrete version (no range), git URL, 40-hex commit OID, 64-hex SHA-256 `content_hash`, optional `subdir`. Sorted by path.

**Cache** (`ModuleCache`): default root is `DRACONIC_MOD_CACHE` or `{workspace}/.draconic/mod-cache`. Layout: `mod/{path segments}/{oid}/` for the tree, `vcs/{path segments}/` for a bare clone. Cache hit skips network. Checkout writes `.draconic-checkout-oid`.

**Hash / integrity**: `content_hash_tree` walks regular files (sorted `/` paths, no symlinks, skips the checkout marker). Ensure and import-resolve recompute the hash and compare checkout OID to the lock pin. Mismatch hard-fails; no silent wrong tree.

**Resolve**: highest matching semver tag → commit OID. Fail closed on empty tags, non-semver-only tags, or no match. v1 lock fill walks direct deps only.

**CLI integration**: `get_package` / `get_package_spec` insert the dep, clone/fetch, resolve tag, checkout, hash, merge lock, write manifest + lock. `mod_tidy` keeps pins that still satisfy the req, fetches missing, prunes unused. `ensure_locked_for_entry` (used by `build`) walks ancestors for a lock, materialises pins by OID (does not float tags). Offline miss tells the user to `get` or drop `--offline`.

**Import resolve**: `looks_like_module_path_import` vs `./` `../`. Longest lock prefix wins; remainder is a subpath. Resolves to a file under the checkout; verifies integrity; rejects paths that leave the package root (package boundary, K06.02). Does not fetch.

**Toolchain pin** (D02): string form is optional (`required = false`); table form may set `required = true`. CLI compares `CARGO_PKG_VERSION` exactly. Invalid manifests are treated as unpinned so Program-only commands do not fail on a broken package file.

Crate modules:

- **lib.rs**: `Manifest` parse/write/validate, default git URL.
- **lock.rs**: lock parse/write/entries.
- **cache.rs**: layout, clone/fetch, checkout.
- **hash.rs**: tree SHA-256 and pin verify.
- **resolve.rs**: tag resolve and direct-deps → lock.
- **get.rs**: `draconic get`.
- **tidy.rs**: `draconic mod tidy`.
- **ensure.rs**: build-time ensure + offline.
- **import_resolve.rs**: module-path imports and package boundary.
- **toolchain.rs**: pin check for an entry path.
- **replace.rs**: `[replace]` git / module / path (K11.02). Implemented in schema and `resolve_git_url`; used when the table is present.
- **auth.rs**: HTTPS token or SSH from env (`DRACONIC_GIT_TOKEN`, `DRACONIC_GIT_TOKEN_USER`, `DRACONIC_GIT_SSH_KEY`). Never stored in manifest or lock. `get` clone uses `GitAuth::from_env` (K11.01).
- **subdir.rs**: monorepo subdirectory from module path vs git URL (K11.03). Lock `subdir` and `checkout_with_subdir` are implemented.
- **proxy.rs**: GOPROXY-shaped `DRACONIC_PROXY` list and `clone_or_fetch_with_proxy` (K11.04). Canonical git URL stays identity. Default `get` / `ensure` call clone/fetch without this proxy list (direct git). Opt-in library surface, not the v1 default path.
- **yank.rs**: advisory file via `DRACONIC_ADVISORY` (K11.05). `get` refuses yanked/retracted versions when configured; unset means yank is not a v1 check.
- **later.rs**: classifies K11 knobs. All-false is the v1 bar. Children are opt-in, never silent v1.

## Trade-offs

Git identity and a lockfile buy reproducible trees without a registry. Direct-deps-only resolve keeps v1 small and can miss nested package graphs until a later pass. Integrity hashing refuses symlinks. Later knobs exist in code so they cannot sneak in as defaults; proxy fetch is implemented but not wired through `get_package`.

## Consequences

[[architecture-linker]] discovers `draconic.lock` from the entry’s ancestors and resolves non-relative specifiers through this crate. Relative imports inside a checkout cannot escape the package root. [[architecture-cli]] `build --offline` is the cache-only switch. [[CONTEXT]] glossary term Linker is the flatten step, not this package manager.
