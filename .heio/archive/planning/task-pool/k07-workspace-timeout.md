---
id: "k07-workspace-timeout"
title: "K07 workspace tests finish"
kind: task
status: completed
mode: afk
blocked-by: []
tags: []
created_at: "2026-09-04T18:48:36Z"
updated_at: "2026-09-04T19:03:28Z"
---

# K07 workspace tests finish

## Blocked by

None.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP K07 work; the `draconic-cli` build tests for auto-fetch of missing locked cache entries, `--offline` cache-only with a fixit on miss, and lock-pin preference (no float when a lock is present) stay green, as do the `draconic-pkg` ensure tests for that same cache/offline/pin surface.

## Context

Roadmap ID **K07** (Build integration: auto-fetch; `--offline`). Review of [[s-k07]] left O3 unmet: `cargo test --workspace` timed out at 120s while O1 (`draconic-cli` build) and O2 (`draconic-pkg` ensure) stayed green. If the hang comes from the K07 change, fix that auto-fetch / `--offline` build surface so both the workspace check and those crate tests hold. Mark K07 `done` only when those tests are green. Not K07.01 `draconic build` auto-fetches missing locked cache entries, K07.02 `draconic build --offline` cache only / fixit on miss, K07.03 build prefers lock pins / does not float versions when lock present, K02 Lockfile (`draconic.lock`) resolved pins, K03 module cache layout / git clone, K05 CLI `draconic get` / `draconic mod tidy`, or K08 integrity verify lock hashes / refuse tampered cache. Do not re-open [[s-k07]].

## Verify

`cargo test --workspace --offline --lib --bins && cargo test -p draconic-cli --offline --test build && cargo test -p draconic-pkg --offline ensure` prints `test result: ok.` and finishes (does not hang). `cargo test -p draconic-cli --test build` still prints `test result: ok.` `cargo test -p draconic-pkg ensure` still prints `test result: ok.` K07 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (K07), `crates/draconic-cli/tests/build.rs`, `crates/draconic-pkg/src/ensure.rs`, auto-fetch / `--offline` / lock-pin build surface as needed to unhang workspace tests after K07

## Links

[[s-k07-workspace-timeout]] [[ticket-170-k07-workspace-timeout]] [[s-k07]]
