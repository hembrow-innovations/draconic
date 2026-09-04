---
id: "d04-workspace-timeout"
title: "D04 workspace tests finish"
kind: task
status: completed
mode: afk
blocked-by: []
tags: []
created_at: "2026-09-04T14:36:43Z"
updated_at: "2026-09-04T14:51:31Z"
---

# D04 workspace tests finish

## Blocked by

None.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP D04 work; the `cross_compile` harness stays green.

## Context

Roadmap ID **D04** (Cross-compile matrix: linux/darwin/windows × amd64/arm64 as available). Review of [[s-d04]] left O2 unmet: `cargo test --workspace` timed out at 120s while O1 (`cross_compile`) stayed green. If the hang comes from the D04 change, fix that linux/darwin/windows × amd64/arm64 (as available) matrix surface so both the workspace check and those integration tests hold. Mark D04 `done` only when those tests are green. Not D04.01 non-host triple LLVM smoke, D04.02 matrix docs + CI jobs, D03 reproducible-build byte identity, D05 strip/LTO, or D01 release binaries + install script. Do not re-open [[s-d04]].

## Verify

`cargo test --workspace --offline --lib --bins && cargo test -p draconic-integration-tests --test cross_compile --offline` prints `test result: ok.` and finishes (does not hang). `cargo test -p draconic-integration-tests --test cross_compile` still prints `test result: ok.` D04 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (D04), `tests/integration/tests/cross_compile.rs`, `crates/draconic-backend-llvm`, linux/darwin/windows × amd64/arm64 (as available) matrix surface as needed to unhang workspace tests after D04

## Links

[[s-d04-workspace-timeout]] [[ticket-133-d04-workspace-timeout]] [[s-d04]]
