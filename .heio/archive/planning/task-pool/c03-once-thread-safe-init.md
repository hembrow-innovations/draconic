---
id: "c03-once-thread-safe-init"
title: "C03 once / thread-safe init surface"
kind: task
status: completed
tags: []
created_at: "2026-09-02T13:07:17Z"
updated_at: "2026-09-02T14:31:00Z"
---

# C03 once / thread-safe init surface

## Done

ROADMAP C03 is implemented test-first on native: thread-safe `once` init (`makeOnce` / `onceRun`) and Runtime-internal mutex only where required; `concurrency/sync` fixtures and runtime crate tests are green and C03 is `done`.

## Context

Roadmap ID **C03** (`once` / thread-safe init; mutex only if Runtime internals need it). C03.01–C03.02 already land the per-class once primitive and internal mutex; this sitting unifies them as one honest native sync surface. Tests under `crates/draconic-runtime` and `tests/conformance` fixtures `concurrency/sync`. Harness `tests/conformance/tests/concurrency_sync.rs`. Mark C03 `done` only when those tests are green. Not C01, C02, C04–C06, a user-facing shared JS heap, or a public mutex Host API.

## Verify

`cargo test -p draconic-conformance --test concurrency_sync` prints `test result: ok.` `cargo test -p draconic-runtime --lib` prints `test result: ok.` Workspace `cargo test --workspace` stays green. C03 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (C03), `tests/conformance/fixtures/concurrency/sync`, `tests/conformance/tests/concurrency_sync.rs`, `crates/draconic-runtime/src/host_once_tests.rs`, `crates/draconic-runtime/src/host_mutex_tests.rs`, `crates/draconic-backend-llvm/src/host_once.rs`, native once/mutex paths as needed for the parent surface

## Links

[[s-c03]] [[ticket-73-c03-once-thread-safe-init-mutex]]
