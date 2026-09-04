---
id: "s-c03-workspace-timeout"
title: "C03 workspace tests finish"
kind: slice
status: met
sprint: "platform"
tags: []
created_at: "2026-09-04T13:35:11Z"
updated_at: "2026-09-04T13:47:52Z"
claimed-by: 625f2e34-5bfd-45f9-ad86-5392ad3f5080
---

# C03 workspace tests finish

## Why

Review of [[s-c03]] left ROADMAP C03 unfinished: O1 (`concurrency_sync`) and O2 (`draconic-runtime --lib`) held, but O3 `cargo test --workspace` timed out at 120s. The concurrency location still needs the C03 Loop to leave the workspace green, not only the sync harness and runtime lib tests.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP C03 work. The `concurrency_sync` harness and `draconic-runtime` lib tests stay green. If the hang comes from the C03 change, fix that once / thread-safe init surface so those checks hold. Mark C03 `done` only when those tests are green.

## Blocked by

None.

## Non-goals

- **Re-opening [[s-c03]]**: that slice stays sealed `failed`
- **C03.01**: `once` / thread-safe init primitive (already `done`)
- **C03.02**: Runtime-internal mutex only where required (already `done`)
- **C01**: Worker / OS thread spawn isolate
- **C02**: Message-passing channels
- **C04**: Parallel `draconic test`
- **C05**: Structured cancellation / timeout helpers
- **C06**: Shared-memory atomics (later; not v1 bar)
- user-facing shared JS heap or a public mutex Host API

## Oracle checklist

- [x] O1: workspace tests finish after the C03 Loop
  CHECK: cargo test --workspace --offline --lib --bins && cargo test -p draconic-conformance --test concurrency_sync --offline
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=236a5c7c3e814666 bytes=93015 at=2026-09-04T13:47:26.283Z

- [x] O2: C03 native `once` init stays locked by the concurrency/sync conformance fixtures
  CHECK: cargo test -p draconic-conformance --test concurrency_sync
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=d8e1d02105e3c9c3 bytes=2945 at=2026-09-04T13:47:26.872Z

- [x] O3: C03 once primitive and Runtime-internal mutex stay locked by the runtime crate tests
  CHECK: cargo test -p draconic-runtime --lib
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=450018efa94dfa51 bytes=8239 at=2026-09-04T13:47:37.315Z

## Pool

Durable links to task-pool ids. Never drop them.

- `[[c03-workspace-timeout]]`

## See also

ROADMAP.md C03, `tests/conformance/tests/concurrency_sync.rs`, `tests/conformance/fixtures/concurrency/sync`, `crates/draconic-runtime/src/host_once_tests.rs`, `crates/draconic-runtime/src/host_mutex_tests.rs`, `crates/draconic-backend-llvm/src/host_once.rs`, docs/adr/0003-gc-runtime-and-dual-worlds.md, CONTEXT.md, [[concurrency]], [[s-c03]], [[ticket-122-c03-workspace-timeout]].
