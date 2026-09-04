---
id: "k04-workspace-timeout"
title: "K04 workspace tests finish"
kind: task
status: completed
mode: afk
blocked-by: []
tags: []
created_at: "2026-09-04T18:36:52Z"
updated_at: "2026-09-04T18:42:04Z"
---

# K04 workspace tests finish

## Blocked by

None.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP K04 work; the `draconic-pkg` resolve tests for version req against semver git tags → highest matching commit OID, fail-closed diagnostics (no match / non-semver-only / empty), and direct-deps → lock pins (v1: direct only) stay green.

## Context

Roadmap ID **K04** (Version resolve: semver tag → commit OID; fail closed). Review of [[s-k04]] left O2 unmet: `cargo test --workspace` timed out at 120s while O1 (`draconic-pkg` resolve) stayed green. If the hang comes from the K04 change, fix that version-resolve surface so both the workspace check and the `draconic-pkg` resolve tests for version req against semver git tags → highest matching commit OID, fail-closed diagnostics (no match / non-semver-only / empty), and direct-deps → lock pins (v1: direct only) hold. Mark K04 `done` only when those tests are green. Not K04.01 resolve version req against git tags / highest matching semver, K04.02 fail closed no match / non-semver-only / empty, K04.03 resolve direct-deps set → lock pins, K03 module cache layout / git clone, K05 CLI `draconic get` / `draconic mod tidy`, K02 lockfile (`draconic.lock`) parse/write surface, or K08 integrity verify. Do not re-open [[s-k04]].

## Verify

`cargo test --workspace --offline --lib --bins && cargo test -p draconic-pkg --offline resolve` prints `test result: ok.` and finishes (does not hang). `cargo test -p draconic-pkg resolve` still prints `test result: ok.` K04 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (K04), `crates/draconic-pkg`, `crates/draconic-pkg/src/resolve.rs`, version-resolve surface as needed to unhang workspace tests after K04

## Links

[[s-k04-workspace-timeout]] [[ticket-168-k04-workspace-timeout]] [[s-k04]]
