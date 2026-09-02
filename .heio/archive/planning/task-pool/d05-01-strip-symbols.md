---
id: "d05-01-strip-symbols"
title: "D05.01 CLI/build flags: strip symbols"
kind: task
status: completed
tags: []
created_at: "2026-09-02T13:14:55Z"
updated_at: "2026-09-02T17:58:00Z"
---

# D05.01 CLI/build flags: strip symbols

## Done

ROADMAP D05.01 is implemented test-first on the native target: CLI/build flags strip symbols from native artifacts, `strip_symbols` CLI tests and `binary_size_strip` integration tests are green, and D05.01 is `done`.

## Context

Roadmap ID **D05.01** (CLI/build flags: strip symbols). Distribution already produces host binaries; this sitting exposes a strip-symbols flag on `draconic build` so a shippable native artifact can drop debug symbols without an after-the-fact `strip(1)` folklore step. Tests under `crates/draconic-cli` lock that the flag is invokable. Tests under `tests/integration` lock that a stripped binary is smaller or lacks the symbols the unstripped build kept. Mark D05.01 `done` only when those tests are green. Not D05 parent remainder, D05.02 LTO, U07 DWARF emit, or D03 reproducible-build byte identity.

## Verify

`cargo test -p draconic-cli --test strip_symbols` prints `test result: ok.` `cargo test -p draconic-integration-tests --test binary_size_strip` prints `test result: ok.` Workspace `cargo test --workspace` stays green. D05.01 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (D05.01), `crates/draconic-cli`, `tests/integration`

## Links

[[s-d05-01]] [[ticket-102-d05-01-cli-build-flags-strip-symbols]]
