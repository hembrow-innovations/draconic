---
id: "s-h12-workspace-timeout"
title: "H12 workspace tests finish"
kind: slice
status: met
sprint: "platform"
tags: []
created_at: "2026-09-04T17:22:50Z"
updated_at: "2026-09-04T17:54:50Z"
claimed-by: 28349043-8a13-436f-b15f-c3d63561928e
---

# H12 workspace tests finish

## Why

Review of [[s-h12]] left ROADMAP H12 unfinished: O1 (`host_ws`) held, but O2 `cargo test --workspace` timed out at 120s. The host-io location still needs the H12 Loop to leave the workspace green, not only the host WebSocket conformance harness.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP H12 work. The host WebSocket conformance harness stays green. If the hang comes from the H12 change, fix that WebSocket handshake, frames, and client echo surface so both checks hold. Mark H12 `done` only when those tests are green.

## Blocked by

None.

## Non-goals

- **Re-opening [[s-h12]]**: that slice stays sealed `failed`
- **H12.01**: WebSocket handshake (HTTP/1.1 upgrade) server-side (already `done`)
- **H12.02**: WebSocket frames: text/binary; close; ping/pong (already `done`)
- **H12.03**: WebSocket client dial + echo e2e (already `done`)
- **H06**: TCP listen/accept/connect/read/write
- **H10**: HTTP/1.1 thin helpers (plaintext)
- **H11**: TLS
- **H13**: HTTP/2
- **H00**: host I/O surface policy

## Oracle checklist

- [x] O1: workspace tests finish after the H12 Loop
  CHECK: cargo test --workspace --offline --lib --bins && cargo test -p draconic-conformance --test host_ws --offline && cargo test -p draconic-runtime --lib --offline
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=a44a6c771db2a444 bytes=102586 at=2026-09-04T17:54:32.822Z

- [x] O2: H12 WebSocket handshake, frames, and client echo stay locked by the host ws conformance tests
  CHECK: cargo test -p draconic-conformance --test host_ws
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=ebaf569d2c089f5e bytes=3323 at=2026-09-04T17:54:34.537Z

## Pool

Durable links to task-pool ids. Never drop them.

- `[[h12-workspace-timeout]]`

## See also

ROADMAP.md H12, `tests/conformance/tests/host_ws.rs`, `tests/conformance/fixtures/host/net/ws`, `crates/draconic-backend-llvm/src/host_ws.rs`, `crates/draconic-backend-llvm/src/host_ws_e2e.rs`, `crates/draconic-runtime`, docs/adr/0008-host-io-sockets-first-http.md, CONTEXT.md, `.heio/planning/locations/host-io.md`, [[s-h12]], [[ticket-158-h12-workspace-timeout]].
