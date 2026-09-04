---
id: "s-h07-workspace-timeout"
title: "H07 workspace tests finish"
kind: slice
status: met
sprint: "platform"
tags: []
created_at: "2026-09-04T16:46:24Z"
updated_at: "2026-09-04T17:06:27Z"
claimed-by: 14e9cbf4-86e2-4665-af03-d59ed03e542c
---

# H07 workspace tests finish

## Why

Review of [[s-h07]] left ROADMAP H07 unfinished: O1 (`host_tcp_async`) and O2 (`draconic-runtime` lib) held, but O3 `cargo test --workspace` timed out at 120s. The host-io location still needs the H07 Loop to leave the workspace green, not only the host async TCP harness and Runtime ABI tests.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP H07 work. The host async TCP conformance harness and the runtime crate tests stay green. If the hang comes from the H07 change, fix that async socket I/O + job queue surface so both checks hold. Mark H07 `done` only when those tests are green.

## Blocked by

None.

## Non-goals

- **Re-opening [[s-h07]]**: that slice stays sealed `failed`
- **H07.01**: Non-blocking readiness; complete via job queue (already `done`)
- **H07.02**: Async accept/connect/read/write → Promises; cancel/close settles cleanly (already `done`)
- **H07.03**: Concurrent connections without starving job queue (already `done`)
- **H06**: TCP listen/accept/connect/read/write (sync sockets-first)
- **H08**: UDP
- **H05**: timers / run-loop wait
- **H10**: HTTP/1.1 helpers on sockets

## Oracle checklist

- [x] O1: workspace tests finish after the H07 Loop
  CHECK: cargo test --workspace --offline --lib --bins && cargo test -p draconic-conformance --test host_tcp_async --offline && cargo test -p draconic-runtime --lib --offline
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=4470ad23b8c0fa57 bytes=102114 at=2026-09-04T17:06:00.870Z

- [x] O2: H07 async TCP Promises and concurrent connections stay locked by the host async conformance tests
  CHECK: cargo test -p draconic-conformance --test host_tcp_async
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=63060dfb46c4a5d3 bytes=3053 at=2026-09-04T17:06:01.832Z

## Pool

Durable links to task-pool ids. Never drop them.

- `[[h07-workspace-timeout]]`

## See also

ROADMAP.md H07, `tests/conformance/tests/host_tcp_async.rs`, `tests/conformance/fixtures/host/net/async`, `crates/draconic-runtime`, `crates/draconic-backend-llvm/src/host_tcp_async.rs`, docs/adr/0008-host-io-sockets-first-http.md, CONTEXT.md, `.heio/planning/locations/host-io.md`, [[s-h07]], [[ticket-153-h07-workspace-timeout]].
