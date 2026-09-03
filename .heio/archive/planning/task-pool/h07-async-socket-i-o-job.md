---
id: "h07-async-socket-i-o-job"
title: "H07 Async socket I/O + job queue"
kind: task
status: completed
tags: []
created_at: "2026-09-02T13:28:38Z"
updated_at: "2026-09-03T05:16:34Z"
---

# H07 Async socket I/O + job queue

## Done

ROADMAP H07 is implemented test-first on native: set sockets non-blocking and complete readiness via the job queue, use async accept/connect/read/write that settle as Promises (cancel/close settles cleanly), and run concurrent connections without starving the job queue; `host/net/async` fixtures and runtime crate tests are green and H07 is `done`.

## Context

Roadmap ID **H07** (Async socket I/O + job queue). H07.01–H07.03 already land non-blocking readiness via the job queue, Promise-shaped accept/connect/read/write with cancel/close, and concurrent connections without starving the queue; this sitting unifies them as one honest async TCP surface on native on top of H06 sockets. Tests under `tests/conformance` fixtures `host/net/async`. Harness `tests/conformance/tests/host_tcp_async.rs` plus `crates/draconic-runtime`. Mark H07 `done` only when those tests are green. Not H06, H08, H05, or H10.

## Verify

`cargo test -p draconic-conformance --test host_tcp_async` prints `test result: ok.` `cargo test -p draconic-runtime --lib` prints `test result: ok.` Workspace `cargo test --workspace` stays green. H07 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (H07), `tests/conformance/fixtures/host/net/async`, `tests/conformance/tests/host_tcp_async.rs`, `crates/draconic-runtime`, `crates/draconic-backend-llvm/src/host_tcp_async.rs`, native async TCP paths as needed for the parent surface

## Links

[[s-h07]] [[ticket-38-h07-async-socket-i-o-job]]
