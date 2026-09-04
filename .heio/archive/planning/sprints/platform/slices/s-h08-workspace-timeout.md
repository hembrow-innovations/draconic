---
id: "s-h08-workspace-timeout"
title: "H08 workspace tests finish"
kind: slice
status: met
sprint: "platform"
tags: []
created_at: "2026-09-04T16:53:19Z"
updated_at: "2026-09-04T17:08:25Z"
claimed-by: 1a28d7df-04ce-4e4a-8a4f-8e3d5f4d3554
---

# H08 workspace tests finish

## Why

Review of [[s-h08]] left ROADMAP H08 unfinished: O1 (`host_udp`) held, but O2 `cargo test --workspace` timed out at 120s. The host-io location still needs the H08 Loop to leave the workspace green, not only the host UDP conformance harness.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP H08 work. The host UDP conformance harness stays green. If the hang comes from the H08 change, fix that UDP surface so both checks hold. Mark H08 `done` only when those tests are green.

## Blocked by

None.

## Non-goals

- **Re-opening [[s-h08]]**: that slice stays sealed `failed`
- **H08.01**: UDP bind; sendto/recvfrom; close (already `done`)
- **H08.02**: UDP loopback e2e (already `done`)
- **H06**: TCP listen/accept/connect/read/write
- **H07**: async socket I/O + job queue
- **H09**: DNS
- **H10**: HTTP/1.1 thin helpers
- **H00**: host I/O surface policy

## Oracle checklist

- [x] O1: workspace tests finish after the H08 Loop
  CHECK: cargo test --workspace --offline --lib --bins && cargo test -p draconic-conformance --test host_udp --offline && cargo test -p draconic-runtime --lib --offline
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=d1c3a8838dcb401e bytes=102093 at=2026-09-04T17:08:14.864Z

- [x] O2: H08 UDP bind/sendto/recvfrom and loopback e2e stay locked by the host udp conformance tests
  CHECK: cargo test -p draconic-conformance --test host_udp
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=ce0aacb5bec87f52 bytes=3031 at=2026-09-04T17:08:15.675Z

## Pool

Durable links to task-pool ids. Never drop them.

- `[[h08-workspace-timeout]]`

## See also

ROADMAP.md H08, `tests/conformance/tests/host_udp.rs`, `tests/conformance/fixtures/host/net/udp`, `crates/draconic-backend-llvm/src/host_udp.rs`, `crates/draconic-runtime`, docs/adr/0008-host-io-sockets-first-http.md, CONTEXT.md, `.heio/planning/locations/host-io.md`, [[s-h08]], [[ticket-154-h08-workspace-timeout]].
