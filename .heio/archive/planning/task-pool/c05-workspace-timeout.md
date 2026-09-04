---
id: "c05-workspace-timeout"
title: "C05 workspace tests finish"
kind: task
status: completed
mode: afk
blocked-by: []
tags: []
created_at: "2026-09-04T13:45:59Z"
updated_at: "2026-09-04T13:50:28Z"
---

# C05 workspace tests finish

## Blocked by

None.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP C05 work; the `concurrency_cancel` harness and `draconic-runtime` lib tests stay green.

## Context

Roadmap ID **C05** (Structured cancellation / timeout helpers on async work (channels + timers)). Review of [[s-c05]] left O3 unmet: `cargo test --workspace` timed out at 120s while `concurrency_cancel` and `draconic-runtime --lib` stayed green. If the hang comes from the C05 change, fix that structured cancellation / timeout surface so those checks hold. Mark C05 `done` only when those tests are green. Not C05.01–C05.02 as separate atoms, C01–C04, C06, Node `AbortController` / `AbortSignal` identity, or a new host timer API beyond H05. Do not re-open [[s-c05]].

## Verify

`cargo test --workspace --offline --lib --bins && cargo test -p draconic-conformance --test concurrency_cancel --offline` prints `test result: ok.` and finishes (does not hang). `cargo test -p draconic-conformance --test concurrency_cancel` and `cargo test -p draconic-runtime --lib` still print `test result: ok.` C05 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (C05), `tests/conformance/tests/concurrency_cancel.rs`, `tests/conformance/fixtures/concurrency/cancel`, `crates/draconic-runtime/src/host_cancel_tests.rs`, `crates/draconic-backend-llvm/src/host_cancel.rs`, structured cancellation / timeout paths as needed to unhang workspace tests after C05

## Links

[[s-c05-workspace-timeout]] [[ticket-124-c05-workspace-timeout]] [[s-c05]]
