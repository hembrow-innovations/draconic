---
id: "d05-binary-size-opts"
title: "D05 Binary size opts: strip / LTO flags documented and testable"
kind: task
status: completed
tags: []
created_at: "2026-09-02T19:12:00Z"
updated_at: "2026-09-02T19:15:00Z"
---

# D05 Binary size opts: strip / LTO flags documented and testable

## Done

ROADMAP D05 is implemented test-first on the native target: docs name the strip and LTO flags, `binary_size` integration and CLI tests lock that those flags are documented and invokable together, and D05 is `done`.

## Context

Roadmap ID **D05** (Binary size opts: strip / LTO flags documented and testable). D05.01 lands strip-symbols and D05.02 lands the LTO size-delta smoke; this sitting unifies them as one honest documented-and-testable size-opt surface. Tests under `tests/integration` (`binary_size`) and `crates/draconic-cli` (`binary_size`). Mark D05 `done` only when those tests are green. Not D05.01, D05.02, D03, or D04.

## Verify

`cargo test -p draconic-integration-tests --test binary_size` prints `test result: ok.` `cargo test -p draconic-cli --test binary_size` prints `test result: ok.` Workspace `cargo test --workspace` stays green. D05 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (D05), `tests/integration/tests/binary_size.rs`, `crates/draconic-cli/tests/binary_size.rs`

## Links

[[s-d05]] [[ticket-101-d05-binary-size-opts-strip-lto]]
