---
id: "d05-01-workspace-timeout"
title: "D05.01 workspace tests finish"
kind: task
status: completed
mode: afk
blocked-by: []
tags: []
created_at: "2026-09-04T14:44:19Z"
updated_at: "2026-09-04T14:58:36Z"
---

# D05.01 workspace tests finish

## Blocked by

None.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP D05.01 work; the `strip_symbols` CLI tests and `binary_size_strip` integration harness stay green.

## Context

Roadmap ID **D05.01** (CLI/build flags: strip symbols). Review of [[s-d05-01]] left O3 unmet: `cargo test --workspace` timed out at 120s while O1 (`strip_symbols`) and O2 (`binary_size_strip`) stayed green. If the hang comes from the D05.01 change, fix that CLI/build strip-symbols surface so both the workspace check and those tests hold. Mark D05.01 `done` only when those tests are green. Not D05 parent remainder (documenting strip and LTO together as one umbrella row), D05.02 LTO (or designed) flag and size-delta smoke, U07 native DWARF debug-info emit, D03 reproducible-build byte identity, or D04 cross-compile matrix. Do not re-open [[s-d05-01]].

## Verify

`cargo test --workspace --offline --lib --bins && cargo test -p draconic-cli --test strip_symbols --offline && cargo test -p draconic-integration-tests --test binary_size_strip --offline` prints `test result: ok.` and finishes (does not hang). `cargo test -p draconic-cli --test strip_symbols` and `cargo test -p draconic-integration-tests --test binary_size_strip` still print `test result: ok.` D05.01 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (D05.01), `crates/draconic-cli`, `tests/integration`, CLI/build strip-symbols surface as needed to unhang workspace tests after D05.01

## Links

[[s-d05-01-workspace-timeout]] [[ticket-134-d05-01-workspace-timeout]] [[s-d05-01]]
