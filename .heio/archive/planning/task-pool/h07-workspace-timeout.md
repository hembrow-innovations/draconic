---
id: "h07-workspace-timeout"
title: "H07 workspace tests finish"
kind: task
status: completed
mode: afk
blocked-by: []
tags: []
created_at: "2026-09-04T16:47:54Z"
updated_at: "2026-09-04T17:00:18Z"
---

# H07 workspace tests finish

## Blocked by

None.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP H07 work; the host async TCP conformance harness and the runtime crate tests stay green.

## Context

Roadmap ID **H07** (Async socket I/O + job queue). Review of [[s-h07]] left O3 unmet: `cargo test --workspace` timed out at 120s while O1 (`host_tcp_async`) and O2 (`draconic-runtime` lib) stayed green. If the hang comes from the H07 change, fix that async socket I/O + job queue surface so the workspace check, the host async TCP harness, and the runtime crate tests hold. Mark H07 `done` only when those tests are green. Not H07.01 Non-blocking readiness / complete via job queue, H07.02 Async accept/connect/read/write → Promises / cancel/close settles cleanly, H07.03 Concurrent connections without starving job queue, H06 TCP listen/accept/connect/read/write (sync sockets-first), H08 UDP, H05 timers / run-loop wait, or H10 HTTP/1.1 helpers on sockets. Do not re-open [[s-h07]].

## Verify

`cargo test --workspace --offline --lib --bins && cargo test -p draconic-conformance --test host_tcp_async --offline && cargo test -p draconic-runtime --lib --offline` prints `test result: ok.` and finishes (does not hang). `cargo test -p draconic-conformance --test host_tcp_async` still prints `test result: ok.` H07 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (H07), `tests/conformance/tests/host_tcp_async.rs`, `tests/conformance/fixtures/host/net/async`, `crates/draconic-runtime`, `crates/draconic-backend-llvm/src/host_tcp_async.rs`, async socket I/O + job queue surface as needed to unhang workspace tests after H07

## Links

[[s-h07-workspace-timeout]] [[ticket-153-h07-workspace-timeout]] [[s-h07]]
