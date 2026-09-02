---
id: "d02-toolchain-version-pin"
title: "D02 toolchain version pin in draconic.toml; CLI enforces or warns"
kind: task
status: completed
tags: []
created_at: "2026-09-02T16:59:09Z"
updated_at: "2026-09-02T17:00:00Z"
---

# D02 toolchain version pin in draconic.toml; CLI enforces or warns

## Done

ROADMAP D02 is implemented test-first on the compiler: `draconic.toml` can carry a required or optional toolchain version pin, the CLI warns or hard-fails when the running toolchain ≠ that pin, `toolchain_pin` tests under `crates/draconic-cli` and `tests/integration` are green, and D02 is `done`.

## Context

Roadmap ID **D02** (Toolchain version pin in `draconic.toml`; CLI enforces or warns). D02.01–D02.02 already land the manifest field and the mismatch path; this sitting unifies them as one honest pin + enforce/warn surface. Tests under `crates/draconic-cli` and `tests/integration` (`toolchain_pin`). Mark D02 `done` only when those tests are green. Not D02.01, D02.02, D01, D03, D04, or D05.

## Verify

`cargo test -p draconic-cli --test toolchain_pin` prints `test result: ok.` `cargo test -p draconic-integration-tests --test toolchain_pin` prints `test result: ok.` Workspace `cargo test --workspace` stays green. D02 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (D02), `crates/draconic-cli/tests/toolchain_pin.rs`, `tests/integration/tests/toolchain_pin.rs`, `crates/draconic-cli/src/toolchain_pin.rs`

## Links

[[s-d02]] [[ticket-94-d02-toolchain-version-pin-in-draconic]]
