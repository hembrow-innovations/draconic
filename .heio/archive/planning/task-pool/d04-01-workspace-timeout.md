---
id: "d04-01-workspace-timeout"
title: "D04.01 workspace tests finish"
kind: task
status: completed
mode: afk
blocked-by: []
tags: []
created_at: "2026-09-04T14:28:57Z"
updated_at: "2026-09-04T14:44:32Z"
---

# D04.01 workspace tests finish

## Blocked by

None.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP D04.01 work; the `cross_compile_non_host` harness stays green.

## Context

Roadmap ID **D04.01** (Cross-compile: at least one non-host triple smoke). Review of [[s-d04-01]] left O2 unmet: `cargo test --workspace` timed out at 120s while O1 (`cross_compile_non_host`) stayed green. If the hang comes from the D04.01 change, fix that non-host triple LLVM smoke surface so both the workspace check and those integration tests hold. Mark D04.01 `done` only when those tests are green. Not D04 parent remainder, D04.02 matrix docs + CI jobs, D03 reproducible-build byte identity, D05 strip/LTO, or D01 release binaries + install script. Do not re-open [[s-d04-01]].

## Verify

`cargo test --workspace --offline --lib --bins && cargo test -p draconic-integration-tests --test cross_compile_non_host --offline` prints `test result: ok.` and finishes (does not hang). `cargo test -p draconic-integration-tests --test cross_compile_non_host` still prints `test result: ok.` D04.01 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (D04.01), `tests/integration/tests/cross_compile_non_host.rs`, `crates/draconic-backend-llvm`, non-host triple LLVM smoke surface as needed to unhang workspace tests after D04.01

## Links

[[s-d04-01-workspace-timeout]] [[ticket-131-d04-01-workspace-timeout]] [[s-d04-01]]
