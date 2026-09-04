---
id: "s-h13-workspace-timeout"
title: "H13 workspace tests finish"
kind: slice
status: met
sprint: "platform"
tags: []
created_at: "2026-09-04T17:31:04Z"
updated_at: "2026-09-04T17:57:15Z"
claimed-by: 769b88e9-732f-4d0d-bcce-0fa2a80d9a98
---

# H13 workspace tests finish

## Why

Review of [[s-h13]] left ROADMAP H13 unfinished: O1 (`host_http2`) held, but O2 `cargo test --workspace` timed out at 120s. The host-io location still needs the H13 Loop to leave the workspace green, not only the host HTTP/2 conformance harness.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP H13 work. The host HTTP/2 conformance harness stays green. If the hang comes from the H13 change, fix that HTTP/2 preface and single-stream request/response surface so both checks hold. Mark H13 `done` only when those tests are green.

## Blocked by

None.

## Non-goals

- **Re-opening [[s-h13]]**: that slice stays sealed `failed`
- **H13.01**: HTTP/2 preface + single stream request/response (already `done`)
- **H06**: TCP listen/accept/connect/read/write
- **H10**: HTTP/1.1 thin helpers (plaintext)
- **H11**: TLS
- **H12**: WebSocket
- **H00**: host I/O surface policy
- multiplexed streams, push, or a full HTTP/2 stack beyond the single-stream helpers

## Oracle checklist

- [x] O1: workspace tests finish after the H13 Loop
  CHECK: cargo test --workspace --offline --lib --bins && cargo test -p draconic-conformance --test host_http2 --offline && cargo test -p draconic-runtime --lib --offline
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=5453e13af828819c bytes=102333 at=2026-09-04T17:56:56.825Z

- [x] O2: H13 HTTP/2 preface and single-stream request/response stay locked by the host http2 conformance tests
  CHECK: cargo test -p draconic-conformance --test host_http2
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=c39701790d5b4e0e bytes=3070 at=2026-09-04T17:56:57.871Z

## Pool

Durable links to task-pool ids. Never drop them.

- `[[h13-workspace-timeout]]`

## See also

ROADMAP.md H13, `tests/conformance/tests/host_http2.rs`, `tests/conformance/fixtures/host/http2`, `crates/draconic-backend-llvm/src/host_http2.rs`, `crates/draconic-runtime`, docs/adr/0008-host-io-sockets-first-http.md, CONTEXT.md, `.heio/planning/locations/host-io.md`, [[s-h13]], [[ticket-159-h13-workspace-timeout]].
