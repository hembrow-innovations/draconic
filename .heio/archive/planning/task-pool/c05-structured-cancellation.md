---
id: "c05-structured-cancellation"
title: "C05 structured cancellation / timeout surface"
kind: task
status: completed
tags: []
created_at: "2026-09-02T13:08:19Z"
updated_at: "2026-09-02T14:55:00Z"
---

# C05 structured cancellation / timeout surface

## Done

ROADMAP C05 is implemented test-first on both targets: cancel token abort (sticky/idempotent) and child-link propagation, plus `withTimeout` / `clearWithTimeout` racing async work vs timer so the loser settles cleanly; `concurrency/cancel` fixtures and runtime crate tests are green and C05 is `done`.

## Context

Roadmap ID **C05** (Structured cancellation / timeout helpers on async work (channels + timers)). C05.01–C05.02 already land the per-class cancel-token and timeout fixtures; this sitting unifies them as one honest cancel/timeout surface on both targets. Tests under `tests/conformance` fixtures `concurrency/cancel`. Harness `tests/conformance/tests/concurrency_cancel.rs`. Mark C05 `done` only when those tests are green. Not C01–C04, C06, Node `AbortController` / `AbortSignal` identity, or a new host timer API beyond H05.

## Verify

`cargo test -p draconic-conformance --test concurrency_cancel` prints `test result: ok.` `cargo test -p draconic-runtime --lib` prints `test result: ok.` Workspace `cargo test --workspace` stays green. C05 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (C05), `tests/conformance/fixtures/concurrency/cancel`, `tests/conformance/tests/concurrency_cancel.rs`, `crates/draconic-runtime/src/host_cancel_tests.rs`, `crates/draconic-backend-llvm/src/host_cancel.rs`, js/native cancel/timeout paths as needed for the parent surface

## Links

[[s-c05]] [[ticket-75-c05-structured-cancellation-timeout-helpers-on]]
