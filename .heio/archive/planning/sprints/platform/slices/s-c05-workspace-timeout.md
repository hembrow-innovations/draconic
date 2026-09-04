---
id: "s-c05-workspace-timeout"
title: "C05 workspace tests finish"
kind: slice
status: met
sprint: "platform"
tags: []
created_at: "2026-09-04T13:44:48Z"
updated_at: "2026-09-04T13:56:03Z"
claimed-by: f1cafc93-7820-4f7d-8a5e-dab101fab0b5
---

# C05 workspace tests finish

## Why

Review of [[s-c05]] left ROADMAP C05 unfinished: O1 (`concurrency_cancel`) and O2 (`draconic-runtime --lib`) held, but O3 `cargo test --workspace` timed out at 120s. The concurrency location still needs the C05 Loop to leave the workspace green, not only the cancel harness and runtime lib tests.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP C05 work. The `concurrency_cancel` harness and `draconic-runtime` lib tests stay green. If the hang comes from the C05 change, fix that structured cancellation / timeout surface so those checks hold. Mark C05 `done` only when those tests are green.

## Blocked by

None.

## Non-goals

- **Re-opening [[s-c05]]**: that slice stays sealed `failed`
- **C05.01**: Cancel token / Abort-like signal: propagate to async work (already `done`)
- **C05.02**: Timeout helper: race work vs timer (already `done`)
- **C01**: Worker / OS thread spawn isolate
- **C02**: Message-passing channels
- **C03**: `once` / thread-safe init
- **C04**: Parallel `draconic test`
- **C06**: Shared-memory atomics (later; not v1 bar)
- Node `AbortController` / `AbortSignal` identity or a new host timer API beyond H05

## Oracle checklist

- [x] O1: workspace tests finish after the C05 Loop
  CHECK: cargo test --workspace --offline --lib --bins && cargo test -p draconic-conformance --test concurrency_cancel --offline
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=3570b717da46838b bytes=93476 at=2026-09-04T13:55:30.063Z

- [x] O2: C05 cancel-token and timeout fixtures stay locked by the concurrency/cancel conformance tests
  CHECK: cargo test -p draconic-conformance --test concurrency_cancel
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=8ad74782cbd8ca0c bytes=3406 at=2026-09-04T13:55:31.513Z

- [x] O3: C05 cancel-token abort/link and timeout race stay locked by the runtime crate tests
  CHECK: cargo test -p draconic-runtime --lib
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=c7839d7b0b908578 bytes=8239 at=2026-09-04T13:55:45.504Z

## Pool

Durable links to task-pool ids. Never drop them.

- `[[c05-workspace-timeout]]`

## See also

ROADMAP.md C05, `tests/conformance/tests/concurrency_cancel.rs`, `tests/conformance/fixtures/concurrency/cancel`, `crates/draconic-runtime/src/host_cancel_tests.rs`, `crates/draconic-backend-llvm/src/host_cancel.rs`, CONTEXT.md, [[concurrency]], [[s-c05]], [[ticket-124-c05-workspace-timeout]].
