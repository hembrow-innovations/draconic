---
id: "d05-02-workspace-timeout"
title: "D05.02 workspace tests finish"
kind: task
status: completed
mode: afk
blocked-by: []
tags: []
created_at: "2026-09-04T14:51:00Z"
updated_at: "2026-09-04T14:54:41Z"
---

# D05.02 workspace tests finish

## Blocked by

None.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP D05.02 work; the `lto_flag` CLI tests and `binary_size_lto` integration harness stay green.

## Context

Roadmap ID **D05.02** (LTO (or designed) flag documented; size delta smoke). Review of [[s-d05-02]] left O3 unmet: `cargo test --workspace` timed out at 120s while O1 (`lto_flag`) and O2 (`binary_size_lto`) stayed green. If the hang comes from the D05.02 change, fix that CLI/build LTO surface so both the workspace check and those tests hold. Mark D05.02 `done` only when those tests are green. Not D05.01 CLI/build flags that strip symbols, D05 parent remainder (documenting strip and LTO together as one umbrella row), D03 reproducible-build byte identity, D04 cross-compile matrix, or U07 native DWARF debug-info emit. Do not re-open [[s-d05-02]].

## Verify

`cargo test --workspace --offline --lib --bins && cargo test -p draconic-cli --test lto_flag --offline && cargo test -p draconic-integration-tests --test binary_size_lto --offline` prints `test result: ok.` and finishes (does not hang). `cargo test -p draconic-cli --test lto_flag` and `cargo test -p draconic-integration-tests --test binary_size_lto` still print `test result: ok.` D05.02 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (D05.02), `crates/draconic-cli`, `tests/integration`, CLI/build LTO surface as needed to unhang workspace tests after D05.02

## Links

[[s-d05-02-workspace-timeout]] [[ticket-135-d05-02-workspace-timeout]] [[s-d05-02]]
