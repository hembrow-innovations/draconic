---
id: "r06-panic-backtraces-with-source-locations"
title: "R06 Panic backtraces with source locations"
kind: task
status: completed
tags: []
created_at: "2026-09-02T14:05:16Z"
updated_at: "2026-09-05T17:45:00Z"
---

# R06 Panic backtraces with source locations

## Done

ROADMAP R06 is implemented test-first on the native target: abort-class faults emit a backtrace from Runtime, and a native panic/abort of a Draconic program reports Draconic source locations via U07 DWARF; runtime and integration tests are green and R06 is `done`.

## Context

Roadmap ID **R06** (Panic backtraces with source locations (ties **U07** DWARF)). U07 already lands DWARF mapping Draconic source lines; this sitting wires the Runtime abort/panic path so a process abort names user source, not only libc frames. Runtime tests under `crates/draconic-runtime` (`--lib backtrace`) lock abort-class backtraces. Integration tests under `tests/integration` (`--test panic_backtrace`) lock Draconic source locations on native panic/abort. Mark R06 `done` only when those tests are green. Not U07 DWARF emit (already `done`), U03 JS source maps, R04 / R04.01 / R04.02 panic-vs-catchable policy, R05 fuzz/stress, or N09 GC stress.

## Verify

`cargo test -p draconic-runtime --lib backtrace` prints `test result: ok.` `cargo test -p draconic-integration-tests --test panic_backtrace` prints `test result: ok.` Workspace `cargo test --workspace` stays green. R06 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (R06), `crates/draconic-runtime`, `tests/integration`, `tests/integration/tests/panic_backtrace.rs`

## Links

[[s-r06]] [[ticket-116-r06-panic-backtraces-with-source-locations]]

## Gauntlet

- **round**: 1
- **command**: cargo test -p draconic-runtime --lib backtrace; cargo test -p draconic-integration-tests --test panic_backtrace; cargo test --workspace
- **result**: win
- **gap**: none
