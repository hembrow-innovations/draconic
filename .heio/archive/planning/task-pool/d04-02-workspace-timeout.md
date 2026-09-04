---
id: "d04-02-workspace-timeout"
title: "D04.02 workspace tests finish"
kind: task
status: completed
mode: afk
blocked-by: []
tags: []
created_at: "2026-09-04T14:33:11Z"
updated_at: "2026-09-04T14:47:49Z"
---

# D04.02 workspace tests finish

## Blocked by

None.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP D04.02 work; the `cross_compile_matrix` harness stays green.

## Context

Roadmap ID **D04.02** (Matrix docs + CI jobs for available OS/arch pairs). Review of [[s-d04-02]] left O2 unmet: `cargo test --workspace` timed out at 120s while O1 (`cross_compile_matrix`) stayed green. If the hang comes from the D04.02 change, fix that matrix docs + CI jobs surface so both the workspace check and those integration tests hold. Mark D04.02 `done` only when those tests are green. Not D04.01 non-host triple LLVM smoke, D04 parent remainder (pairs that are not available), D05 strip/LTO, D01 release binaries + install script, or D03 reproducible-build byte identity. Do not re-open [[s-d04-02]].

## Verify

`cargo test --workspace --offline --lib --bins && cargo test -p draconic-integration-tests --test cross_compile_matrix --offline` prints `test result: ok.` and finishes (does not hang). `cargo test -p draconic-integration-tests --test cross_compile_matrix` still prints `test result: ok.` D04.02 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (D04.02), `tests/integration/tests/cross_compile_matrix.rs`, `.github/workflows`, matrix docs + CI jobs surface as needed to unhang workspace tests after D04.02

## Links

[[s-d04-02-workspace-timeout]] [[ticket-132-d04-02-workspace-timeout]] [[s-d04-02]]
