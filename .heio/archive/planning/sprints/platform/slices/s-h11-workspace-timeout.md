---
id: "s-h11-workspace-timeout"
title: "H11 workspace tests finish"
kind: slice
status: met
sprint: "platform"
tags: []
created_at: "2026-09-04T17:18:30Z"
updated_at: "2026-09-04T17:44:19Z"
claimed-by: f613ea5d-22c8-4616-8e30-d2e3158707fc
---

# H11 workspace tests finish

## Why

Review of [[s-h11]] left ROADMAP H11 unfinished: O1 (`host_tls`) held, but O2 `cargo test --workspace` timed out at 120s. The host-io location still needs the H11 Loop to leave the workspace green, not only the host TLS conformance harness.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP H11 work. The host TLS conformance harness stays green. If the hang comes from the H11 change, fix that TLS client/server wrap and HTTPS loopback surface so both checks hold. Mark H11 `done` only when those tests are green.

## Blocked by

None.

## Non-goals

- **Re-opening [[s-h11]]**: that slice stays sealed `failed`
- **H11.01**: TLS client wrap trust roots / insecure-test (already `done`)
- **H11.02**: TLS server wrap cert/key handshake (already `done`)
- **H11.03**: HTTPS HTTP/1.1 over TLS loopback (already `done`)
- **H06**: TCP listen/accept/connect/read/write
- **H10**: HTTP/1.1 thin helpers (plaintext)
- **H12**: WebSocket
- **H13**: HTTP/2
- **H00**: host I/O surface policy

## Oracle checklist

- [x] O1: workspace tests finish after the H11 Loop
  CHECK: cargo test --workspace --offline --lib --bins && cargo test -p draconic-conformance --test host_tls --offline && cargo test -p draconic-runtime --lib --offline
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=01f78f1797ed26ce bytes=102524 at=2026-09-04T17:43:58.965Z

- [x] O2: H11 TLS client/server wrap and HTTPS loopback stay locked by the host tls conformance tests
  CHECK: cargo test -p draconic-conformance --test host_tls
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=9d8f81f47375ffcd bytes=3261 at=2026-09-04T17:44:03.959Z

## Pool

Durable links to task-pool ids. Never drop them.

- `[[h11-workspace-timeout]]`

## See also

ROADMAP.md H11, `tests/conformance/tests/host_tls.rs`, `tests/conformance/fixtures/host/net/tls`, `crates/draconic-backend-llvm/src/host_tcp.rs`, `crates/draconic-backend-llvm/src/host_http_server.rs`, `crates/draconic-runtime`, docs/adr/0008-host-io-sockets-first-http.md, CONTEXT.md, `.heio/planning/locations/host-io.md`, [[s-h11]], [[ticket-157-h11-workspace-timeout]].
