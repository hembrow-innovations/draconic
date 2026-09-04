---
id: "s-c02-workspace-timeout"
title: "C02 workspace tests finish"
kind: slice
status: met
sprint: "platform"
tags: []
created_at: "2026-09-04T13:30:40Z"
updated_at: "2026-09-04T13:46:11Z"
claimed-by: 6c0aa67c-962d-4b1a-8bc8-b3307258b514
---

# C02 workspace tests finish

## Why

Review of [[s-c02]] left ROADMAP C02 unfinished: O1 (`concurrency_channels`) held, but O2 `cargo test --workspace` timed out at 120s. The concurrency location still needs the C02 Loop to leave the workspace green, not only the channels harness.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP C02 work. The `concurrency_channels` harness stays green. If the hang comes from the C02 change, fix that channel send/recv surface so both checks hold. Mark C02 `done` only when those tests are green.

## Blocked by

None.

## Non-goals

- **Re-opening [[s-c02]]**: that slice stays sealed `failed`
- **C02.01–C02.04**: scalar/string send/recv, structured-clone, bounded buffer, and worker e2e children already `done`
- **C01**: Worker / OS thread spawn isolate
- **C03**: `once` / thread-safe init
- **C04**: Parallel `draconic test`
- **C05**: Structured cancellation / timeout helpers
- **C06**: Shared-memory atomics (later; not v1 bar)
- user-facing shared JS heap across isolates

## Oracle checklist

- [x] O1: workspace tests finish after the C02 Loop
  CHECK: cargo test --workspace --offline --lib --bins && cargo test -p draconic-conformance --test concurrency_channels --offline
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=98b1ba90f01c3409 bytes=93884 at=2026-09-04T13:45:34.582Z

- [x] O2: C02 send/recv, clone policy, bounded buffer, and worker e2e fixtures stay green on the declared both targets through the channels harness
  CHECK: cargo test -p draconic-conformance --test concurrency_channels
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=e03de6d22d653d22 bytes=3814 at=2026-09-04T13:45:39.178Z

## Pool

Durable links to task-pool ids. Never drop them.

- `[[c02-workspace-timeout]]`

## See also

ROADMAP.md C02, `tests/conformance/tests/concurrency_channels.rs`, `tests/conformance/fixtures/concurrency/channels`, `crates/draconic-backend-llvm/src/host_channels.rs`, `crates/draconic-backend-llvm/src/host_worker_channels.rs`, CONTEXT.md, [[concurrency]], [[s-c02]], [[ticket-121-c02-workspace-timeout]].
