---
id: "h14-signals"
title: "H14 Signals"
kind: task
status: completed
tags: []
created_at: "2026-09-02T13:32:00Z"
updated_at: "2026-09-03T05:16:34Z"
---

# H14 Signals

## Done

ROADMAP H14 is implemented test-first on native: watch SIGINT/SIGTERM into a handler/job (default terminate stays documented), ignore a signal, and restore the default disposition; `host/process` signal fixtures and runtime `host_signal` tests are green and H14 is `done`.

## Context

Roadmap ID **H14** (Signals). H14.01–H14.02 already land SIGINT/SIGTERM watch into a handler/job with default terminate documented, plus ignore and restore default; this sitting unifies them as one honest native signal surface. Tests under `tests/conformance` fixtures `host/process` (signal fixtures) and `crates/draconic-runtime`. Harness `tests/conformance/tests/host_process.rs`. Mark H14 `done` only when those tests are green. Not H01, H15, H16, or H00.

## Verify

`cargo test -p draconic-conformance --test host_process signal` prints `test result: ok.` `cargo test -p draconic-runtime host_signal` prints `test result: ok.` Workspace `cargo test --workspace` stays green. H14 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (H14), `tests/conformance/fixtures/host/process`, `tests/conformance/tests/host_process.rs`, `crates/draconic-backend-llvm/src/host_signals.rs`, `crates/draconic-runtime`

## Links

[[s-h14]] [[ticket-45-h14-signals]]
