---
id: "s-c01-workspace-timeout"
title: "C01 workspace tests finish"
kind: slice
status: met
sprint: "platform"
tags: []
created_at: "2026-09-04T13:26:18Z"
updated_at: "2026-09-04T13:39:54Z"
claimed-by: 802ebcd7-7ce7-4d46-ab2e-5406a79e938b
---

# C01 workspace tests finish

## Why

Review of [[s-c01]] left ROADMAP C01 unfinished: O1 (`concurrency_workers`) held, but O2 `cargo test --workspace` timed out at 120s. The concurrency location still needs the C01 Loop to leave the workspace green, not only the workers harness.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP C01 work. The `concurrency_workers` harness stays green. If the hang comes from the C01 change, fix that worker/isolate surface so both checks hold. Mark C01 `done` only when those tests are green.

## Blocked by

None.

## Non-goals

- **Re-opening [[s-c01]]**: that slice stays sealed `failed`
- **C01.01–C01.04**: spawn/join/terminate/OS-thread children already `done`
- **C02**: Message-passing channels
- **C03**: `once` / thread-safe init
- **C04**: Parallel `draconic test`
- **C05**: Structured cancellation / timeout helpers
- **C06**: Shared-memory atomics (later; not v1 bar)
- Node/Web Worker API identity or a shared JS heap

## Oracle checklist

- [x] O1: workspace tests finish after the C01 Loop
  CHECK: cargo test --workspace --offline --lib --bins && cargo test -p draconic-conformance --test concurrency_workers --offline
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=7722410c71884147 bytes=93611 at=2026-09-04T13:39:32.923Z

- [x] O2: C01 spawn/join/terminate and isolate-heap fixtures stay green on the declared both targets through the workers harness
  CHECK: cargo test -p draconic-conformance --test concurrency_workers
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=029755a6db3c9eba bytes=3541 at=2026-09-04T13:39:34.513Z

## Pool

Durable links to task-pool ids. Never drop them.

- [[c01-workspace-timeout]]

## See also

ROADMAP.md C01, `tests/conformance/tests/concurrency_workers.rs`, `tests/conformance/fixtures/concurrency/workers`, `crates/draconic-runtime`, docs/adr/0003-gc-runtime-and-dual-worlds.md, CONTEXT.md, [[concurrency]], [[s-c01]], [[ticket-120-c01-workspace-timeout]].
