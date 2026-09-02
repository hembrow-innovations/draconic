---
id: "d04-cross-compile-matrix"
title: "D04 cross-compile matrix surface"
kind: task
status: completed
tags: []
created_at: "2026-09-02T13:57:24Z"
updated_at: "2026-09-02T17:57:37Z"
---

# D04 cross-compile matrix surface

## Done

ROADMAP D04 is implemented test-first on the native target: the toolchain emits for the available linux/darwin/windows × amd64/arm64 pairs (unavailable pairs stay out of the sitting); `cross_compile` integration tests lock that combined matrix, including LLVM backend emit, and D04 is `done`.

## Context

Roadmap ID **D04** (Cross-compile matrix: linux/darwin/windows × amd64/arm64 (as available)). D04.01 is the non-host triple LLVM smoke and D04.02 is matrix docs plus CI jobs; this sitting unifies them as one honest available-OS/arch compile surface. Tests under `tests/integration` (`cross_compile`). Mark D04 `done` only when those tests are green. Not D04.01, D04.02, D03, D05, or D01.

## Verify

`cargo test -p draconic-integration-tests --test cross_compile` prints `test result: ok.` Workspace `cargo test --workspace` stays green. D04 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (D04), `tests/integration/tests/cross_compile.rs`, `crates/draconic-backend-llvm`

## Links

[[s-d04]] [[ticket-98-d04-cross-compile-matrix-linux-darwin]]
