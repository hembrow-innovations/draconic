---
id: "s-c06-workspace-timeout"
title: "C06 workspace tests finish"
kind: slice
status: met
sprint: "platform"
tags: []
created_at: "2026-09-04T13:54:46Z"
updated_at: "2026-09-04T14:07:46Z"
claimed-by: 8655b2dc-e52d-4412-9717-2c68c085e6ea
---

# C06 workspace tests finish

## Why

Review of [[s-c06]] left ROADMAP C06 unfinished: O1 (`concurrency_atomics`) held, but O2 `cargo test --workspace` timed out at 120s. The concurrency location still needs the C06 Loop to leave the workspace green, not only the atomics harness.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP C06 work. The `concurrency_atomics` harness stays green. If the hang comes from the C06 change, fix that shared-memory atomics surface so both checks hold. Mark C06 `done` only when those tests are green.

## Blocked by

None.

## Non-goals

- **Re-opening [[s-c06]]**: that slice stays sealed `failed`
- **C01**: Worker / OS thread spawn isolate
- **C02**: Message-passing channels
- **C03**: `once` / thread-safe init
- **C04**: Parallel `draconic test`
- **C05**: Structured cancellation / timeout helpers
- full ECMA-262 `SharedArrayBuffer` / `Atomics` Test262 allowlist (E19 / S02)
- Node `worker_threads` SharedArrayBuffer identity or a shared JS heap

## Oracle checklist

- [x] O1: workspace tests finish after the C06 Loop
  CHECK: cargo test --workspace --offline --lib --bins && cargo test -p draconic-conformance --test concurrency_atomics --offline
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=516a862f2d139e67 bytes=93251 at=2026-09-04T14:07:37.320Z

- [x] O2: C06 shared-memory atomics stay locked by the concurrency/atomics conformance tests
  CHECK: cargo test -p draconic-conformance --test concurrency_atomics
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=7a496aa6881871d3 bytes=3181 at=2026-09-04T14:07:38.716Z

## Pool

Durable links to task-pool ids. Never drop them.

- `[[c06-workspace-timeout]]`

## See also

ROADMAP.md C06, `tests/conformance/tests/concurrency_atomics.rs`, `tests/conformance/fixtures/concurrency/atomics`, `crates/draconic-runtime`, `crates/draconic-backend-llvm`, `crates/draconic-check/src/host_api.rs`, CONTEXT.md, [[concurrency]], [[s-c06]], [[ticket-125-c06-workspace-timeout]].
