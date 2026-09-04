---
id: "s-h10-workspace-timeout"
title: "H10 workspace tests finish"
kind: slice
status: met
sprint: "platform"
tags: []
created_at: "2026-09-04T17:14:00Z"
updated_at: "2026-09-04T17:36:49Z"
claimed-by: f744220a-fcdb-4be6-9386-5c03e63d49d5
---

# H10 workspace tests finish

## Why

Review of [[s-h10]] left ROADMAP H10 unfinished: O1 (`host_http`) held, but O2 `cargo test --workspace` timed out at 120s. The host-io location still needs the H10 Loop to leave the workspace green, not only the host HTTP/1.1 conformance harness.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP H10 work. The host HTTP/1.1 conformance harness stays green. If the hang comes from the H10 change, fix that HTTP/1.1 thin-helper surface so both checks hold. Mark H10 `done` only when those tests are green.

## Blocked by

None.

## Non-goals

- **Re-opening [[s-h10]]**: that slice stays sealed `failed`
- **H10.01**: HTTP/1.1 request parse: line + headers + bounded body (already `done`)
- **H10.02**: HTTP/1.1 response write: status + headers + body (already `done`)
- **H10.03**: HTTP/1.1 server one-shot (already `done`)
- **H10.04**: HTTP/1.1 keep-alive optional (already `done`)
- **H10.05**: HTTP/1.1 client on connected TCP (already `done`)
- **H10.06**: Chunked transfer encoding (already `done`)
- **H10.07**: HTTP listen helpers hard-error on js (already `done`)
- **H06**: TCP listen/accept/connect/read/write
- **H11**: TLS
- **H12**: WebSocket
- **H13**: HTTP/2
- **H17**: Success Programs & host cutover

## Oracle checklist

- [x] O1: workspace tests finish after the H10 Loop
  CHECK: cargo test --workspace --offline --lib --bins && cargo test -p draconic-conformance --test host_http --offline && cargo test -p draconic-runtime --lib --offline
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=c15051b65433a972 bytes=102745 at=2026-09-04T17:36:23.570Z

- [x] O2: H10 HTTP/1.1 parse/write/server/client/chunked stay locked by the host http conformance tests
  CHECK: cargo test -p draconic-conformance --test host_http
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=4a4191f8a1293dde bytes=3482 at=2026-09-04T17:36:25.825Z

## Pool

Durable links to task-pool ids. Never drop them.

- `[[h10-workspace-timeout]]`

## See also

ROADMAP.md H10, `tests/conformance/tests/host_http.rs`, `tests/conformance/fixtures/host/http`, `crates/draconic-backend-llvm/src/host_http.rs`, `crates/draconic-backend-llvm/src/host_http_server.rs`, `crates/draconic-runtime`, docs/adr/0008-host-io-sockets-first-http.md, CONTEXT.md, `.heio/planning/locations/host-io.md`, [[s-h10]], [[ticket-156-h10-workspace-timeout]].
