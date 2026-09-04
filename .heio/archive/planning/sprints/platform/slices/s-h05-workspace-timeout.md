---
id: "s-h05-workspace-timeout"
title: "H05 workspace tests finish"
kind: slice
status: met
sprint: "platform"
tags: []
created_at: "2026-09-04T16:28:56Z"
updated_at: "2026-09-04T16:59:56Z"
claimed-by: 4e3db10a-9b89-428c-99aa-df29d055136b
---

# H05 workspace tests finish

## Why

Review of [[s-h05]] left ROADMAP H05 unfinished: O1 (`host_time`) held, but O2 `cargo test --workspace` timed out at 120s. The host-io location still needs the H05 Loop to leave the workspace green, not only the host time conformance harness.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP H05 work. The host time conformance harness stays green. If the hang comes from the H05 change, fix that time, clock, and job-queue timer surface so both checks hold. Mark H05 `done` only when those tests are green.

## Blocked by

None.

## Non-goals

- **Re-opening [[s-h05]]**: that slice stays sealed `failed`
- **H05.01**: wall clock `Date.now` / host `now_ms` (already `done`)
- **H05.02**: monotonic clock for durations (already `done`)
- **H05.03**: `setTimeout` / `clearTimeout` via job queue (already `done`)
- **H05.04**: `setInterval` / `clearInterval` (already `done`)
- **H05.05**: run loop waits for due timers (already `done`)
- **H06**: TCP sockets
- **C05**: structured cancellation / timeout helpers (needs H05, separate row)
- **H00**: host I/O surface policy

## Oracle checklist

- [x] O1: workspace tests finish after the H05 Loop
  CHECK: cargo test --workspace --offline --lib --bins && cargo test -p draconic-conformance --test host_time --offline && cargo test -p draconic-runtime --lib --offline
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=ae4de8f08f762c97 bytes=102412 at=2026-09-04T16:59:44.099Z

- [x] O2: H05 time, clock, and job-queue timers stay locked by the host time conformance tests
  CHECK: cargo test -p draconic-conformance --test host_time
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=0b9f214b308f63d5 bytes=3350 at=2026-09-04T16:59:45.315Z

## Pool

Durable links to task-pool ids. Never drop them.

- `[[h05-workspace-timeout]]`

## See also

ROADMAP.md H05, `tests/conformance/tests/host_time.rs`, `tests/conformance/fixtures/host/time`, `crates/draconic-backend-llvm/src/host_time.rs`, `crates/draconic-backend-llvm/src/host_timers.rs`, `crates/draconic-runtime`, docs/adr/0008-host-io-sockets-first-http.md, CONTEXT.md, `.heio/planning/locations/host-io.md`, [[s-h05]], [[ticket-151-h05-workspace-timeout]].
