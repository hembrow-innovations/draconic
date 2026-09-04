---
id: "k05-workspace-timeout"
title: "K05 workspace tests finish"
kind: task
status: completed
mode: afk
blocked-by: []
tags: []
created_at: "2026-09-04T18:41:41Z"
updated_at: "2026-09-04T18:45:58Z"
---

# K05 workspace tests finish

## Blocked by

None.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP K05 work; the `draconic-cli` get tests for `draconic get <module_path>@<ver>` (fetch, update manifest+lock+cache) and the `draconic-cli` mod_tidy tests for `draconic mod tidy` (lock matches manifest; fetch missing; prune unused) stay green.

## Context

Roadmap ID **K05** (CLI: `draconic get`, `draconic mod tidy`). Review of [[s-k05]] left O3 unmet: `cargo test --workspace` timed out at 120s while O1 (`draconic-cli` get) and O2 (`draconic-cli` mod_tidy) stayed green. If the hang comes from the K05 change, fix that get/tidy CLI surface so both the workspace check and those crate tests hold. Mark K05 `done` only when those tests are green. Not K05.01 `draconic get <module_path>@<ver>` fetch/update manifest+lock+cache, K05.02 `draconic mod tidy` lock matches manifest / fetch missing / prune unused, K01 Manifest (`draconic.toml`), K02 Lockfile (`draconic.lock`), K03 module cache layout / git clone, K04 version resolve (semver tag → commit OID), K07 build integration auto-fetch / offline, or K08 integrity verify lock hashes / refuse tampered cache. Do not re-open [[s-k05]].

## Verify

`cargo test --workspace --offline --lib --bins && cargo test -p draconic-cli --offline --test get && cargo test -p draconic-cli --offline --test mod_tidy` prints `test result: ok.` and finishes (does not hang). `cargo test -p draconic-cli --test get` still prints `test result: ok.` `cargo test -p draconic-cli --test mod_tidy` still prints `test result: ok.` K05 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (K05), `crates/draconic-cli`, `crates/draconic-cli/tests/get.rs`, `crates/draconic-cli/tests/mod_tidy.rs`, `crates/draconic-pkg/src/get.rs`, `crates/draconic-pkg/src/tidy.rs`, get/tidy CLI surface as needed to unhang workspace tests after K05

## Links

[[s-k05-workspace-timeout]] [[ticket-169-k05-workspace-timeout]] [[s-k05]]
