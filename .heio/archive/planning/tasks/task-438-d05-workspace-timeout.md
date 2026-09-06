---
id: "task-438-d05-workspace-timeout"
title: "D05 workspace tests finish"
kind: task
status: completed
mode: afk
blocked_by: []
tags: []
created_at: "2026-09-04T15:00:34Z"
updated_at: "2026-09-06T18:00:00Z"
sprint: "platform"
---

# D05 workspace tests finish

## Blocked by

None.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP D05 work; the `binary_size` integration harness stays green.

## Context

Roadmap ID **D05** (Binary size opts: strip / LTO flags documented and testable). Review of [[slice-263-d05]] left O2 unmet: `cargo test --workspace` timed out at 120s while O1 (`binary_size`) stayed green. If the hang comes from the D05 change, fix that strip / LTO flags surface so both the workspace check and those integration tests hold. Mark D05 `done` only when those tests are green. Not D05.01 CLI/build flags that strip symbols, D05.02 LTO (or designed) flag and size-delta smoke, D03 reproducible-build byte identity, D04 cross-compile matrix, or U07 native DWARF debug-info emit. Do not re-open [[slice-263-d05]].

## Verify

`cargo test --workspace --offline --lib --bins && cargo test -p draconic-integration-tests --test binary_size --offline` prints `test result: ok.` and finishes (does not hang). `cargo test -p draconic-integration-tests --test binary_size` still prints `test result: ok.` D05 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (D05), `tests/integration`, `crates/draconic-cli`, strip / LTO flags surface as needed to unhang workspace tests after D05

## Links

[[slice-262-d05-workspace-timeout]] [[ticket-136-d05-workspace-timeout]] [[slice-263-d05]]
