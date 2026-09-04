---
id: "d02-workspace-timeout"
title: "D02 workspace tests finish"
kind: task
status: completed
mode: afk
blocked-by: []
tags: []
created_at: "2026-09-04T14:07:52Z"
updated_at: "2026-09-04T14:11:55Z"
---

# D02 workspace tests finish

## Blocked by

None.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP D02 work; the CLI and integration `toolchain_pin` harnesses stay green.

## Context

Roadmap ID **D02** (Toolchain version pin in `draconic.toml`; CLI enforces or warns). Review of [[s-d02]] left O3 unmet: `cargo test --workspace` timed out at 120s while O1–O2 (`draconic-cli` and `draconic-integration-tests` `toolchain_pin`) stayed green. If the hang comes from the D02 change, fix that toolchain version pin in `draconic.toml` (CLI enforces or warns) so both the workspace check and those pin tests hold. Mark D02 `done` only when those tests are green. Not D02.01–D02.02 as separate atoms, D01 release binaries + install script, D03 reproducible-build identity, D04 cross-compile matrix, or D05 strip/LTO. Do not re-open [[s-d02]].

## Verify

`cargo test --workspace --offline --lib --bins && cargo test -p draconic-cli --test toolchain_pin --offline && cargo test -p draconic-integration-tests --test toolchain_pin --offline` prints `test result: ok.` and finishes (does not hang). `cargo test -p draconic-cli --test toolchain_pin` and `cargo test -p draconic-integration-tests --test toolchain_pin` still print `test result: ok.` D02 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (D02), `crates/draconic-cli/tests/toolchain_pin.rs`, `tests/integration/tests/toolchain_pin.rs`, `crates/draconic-cli/src/toolchain_pin.rs`, `crates/draconic-pkg/src/toolchain.rs`, toolchain version pin paths as needed to unhang workspace tests after D02

## Links

[[s-d02-workspace-timeout]] [[ticket-127-d02-workspace-timeout]] [[s-d02]]
