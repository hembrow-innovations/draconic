---
id: "k05-cli-draconic-get-draconic-mod"
title: "K05 CLI: `draconic get`, `draconic mod tidy`"
kind: task
status: completed
tags: []
created_at: "2026-09-02T13:35:41Z"
updated_at: "2026-09-03T05:16:34Z"
---

# K05 CLI: `draconic get`, `draconic mod tidy`

## Done

ROADMAP K05 is implemented test-first on the compiler target: `draconic get` fetches a module path@version and writes manifest, lock, and cache; `draconic mod tidy` makes the lock match the manifest, fetches missing deps, and prunes unused pins; `draconic-cli` get and mod_tidy tests are green and K05 is `done`.

## Context

Roadmap ID **K05** (CLI: `draconic get`, `draconic mod tidy`). K05.01–K05.02 already land `draconic get <module_path>@<ver>` (fetch, update manifest+lock+cache) and `draconic mod tidy` (lock matches manifest; fetch missing; prune unused); this sitting unifies them as one honest get/tidy CLI on the compiler target. Tests in `crates/draconic-cli` (`get`, `mod_tidy`). Mark K05 `done` only when those tests are green. Not K05.01–K05.02 as separate atoms, K01, K02, K03, K04, K07, or K08.

## Verify

`cargo test -p draconic-cli --test get` prints `test result: ok.` `cargo test -p draconic-cli --test mod_tidy` prints `test result: ok.` Workspace `cargo test --workspace` stays green. K05 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (K05), `crates/draconic-cli`, `crates/draconic-cli/tests/get.rs`, `crates/draconic-cli/tests/mod_tidy.rs`, `crates/draconic-pkg/src/get.rs`, `crates/draconic-pkg/src/tidy.rs`

## Links

[[s-k05]] [[ticket-54-k05-cli-draconic-get-draconic-mod]]
