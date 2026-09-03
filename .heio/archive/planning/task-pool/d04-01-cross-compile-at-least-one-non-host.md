---
id: "d04-01-cross-compile-at-least-one-non-host"
title: "D04.01 Cross-compile: at least one non-host triple smoke"
kind: task
status: completed
tags: []
created_at: "2026-09-02T22:08:22Z"
updated_at: "2026-09-03T05:16:34Z"
---

# D04.01 Cross-compile: at least one non-host triple smoke

## Done

ROADMAP D04.01 is implemented test-first on the native target: compiling a Program through the LLVM backend succeeds for at least one non-host triple from the D04 matrix; tests under `tests/integration` and `crates/draconic-backend-llvm` lock that smoke, and D04.01 is `done`.

## Context

Roadmap ID **D04.01** (Cross-compile: at least one non-host triple smoke). Parent D04 names the available linux/darwin/windows × amd64/arm64 compile surface and does not require a non-host success; this sitting proves LLVM emit for at least one triple that is not the host. Tests under `tests/integration` and `crates/draconic-backend-llvm`. Harness `cargo test -p draconic-integration-tests --test cross_compile_non_host`. Mark D04.01 `done` only when those tests are green. Not D04 parent remainder, D04.02 matrix docs/CI, D03 reproducible-build identity, D05 strip/LTO, or D01 release binaries.

## Verify

`cargo test -p draconic-integration-tests --test cross_compile_non_host` prints `test result: ok.` Workspace `cargo test --workspace` stays green. D04.01 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (D04.01), `tests/integration`, `crates/draconic-backend-llvm`

## Links

[[s-d04-01]] [[ticket-99-d04-01-cross-compile-at-least-one]]
