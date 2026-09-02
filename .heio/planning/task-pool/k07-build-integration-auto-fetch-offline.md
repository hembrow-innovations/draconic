---
id: "k07-build-integration-auto-fetch-offline"
title: "K07 Build integration: auto-fetch; `--offline`"
kind: task
status: ready
tags: []
created_at: "2026-09-02T13:37:09Z"
updated_at: "2026-09-02T13:37:09Z"
---

# K07 Build integration: auto-fetch; `--offline`

## Done

ROADMAP K07 is implemented test-first on the compiler target: `draconic build` materialises missing locked package checkouts before compile; `draconic build --offline` consults cache only and hard-fails with a `draconic get` / without-`--offline` fixit on miss; when a lock is present, build uses those pins and does not float versions; `draconic-cli` build tests and `draconic-pkg` ensure tests are green and K07 is `done`.

## Context

Roadmap ID **K07** (Build integration: auto-fetch; `--offline`). K07.01–K07.03 already land auto-fetch of missing locked cache entries, `--offline` cache-only with fixit on miss, and lock-pin preference (no float when a lock is present); this sitting unifies them as one honest auto-fetch / `--offline` build surface on the compiler target. Tests in `crates/draconic-cli` (`build`) and `crates/draconic-pkg` (`ensure`). Mark K07 `done` only when those tests are green. Not K07.01–K07.03 as separate atoms, K02, K03, K05, or K08.

## Verify

`cargo test -p draconic-cli --test build` prints `test result: ok.` `cargo test -p draconic-pkg ensure` prints `test result: ok.` Workspace `cargo test --workspace` stays green. K07 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (K07), `crates/draconic-cli`, `crates/draconic-cli/tests/build.rs`, `crates/draconic-pkg`, `crates/draconic-pkg/src/ensure.rs`

## Links

[[s-k07]] [[ticket-55-k07-build-integration-auto-fetch-offline]]
