---
id: "s-h06-workspace-timeout"
title: "H06 workspace tests finish"
kind: slice
status: met
sprint: "platform"
tags: []
created_at: "2026-09-04T16:37:51Z"
updated_at: "2026-09-04T16:57:10Z"
claimed-by: e675aa5b-e5cb-4549-97e8-6a8f2450dbec
---

# H06 workspace tests finish

## Why

Review of [[s-h06]] left ROADMAP H06 unfinished: O1 (`host_tcp`) held, but O2 `cargo test --workspace` timed out at 120s. The host-io location still needs the H06 Loop to leave the workspace green, not only the host TCP conformance harness.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP H06 work. The host TCP conformance harness stays green. If the hang comes from the H06 change, fix that TCP sockets (sockets-first) surface so both checks hold. Mark H06 `done` only when those tests are green.

## Blocked by

None.

## Non-goals

- **Re-opening [[s-h06]]**: that slice stays sealed `failed`
- **H06.01**: TCP listen bind/backlog/close/ephemeral (already `done`)
- **H06.02**: TCP accept → connection handle; peer address (already `done`)
- **H06.03**: TCP connect dial host:port; refused/timeout (already `done`)
- **H06.04**: TCP read/write bytes; partial read; close/shutdown (already `done`)
- **H06.05**: TCP loopback e2e echo (already `done`)
- **H06.06**: TCP listen/accept hard-error on js (already `done`)
- **H07**: async socket I/O + job queue
- **H08**: UDP
- **H00**: host I/O surface policy

## Oracle checklist

- [x] O1: workspace tests finish after the H06 Loop
  CHECK: cargo test --workspace --offline --lib --bins && cargo test -p draconic-conformance --test host_tcp --offline && cargo test -p draconic-runtime --lib --offline
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=d5fc8ab59c4c4e98 bytes=102260 at=2026-09-04T16:56:51.334Z

- [x] O2: H06 TCP sockets stay locked by the host tcp conformance tests
  CHECK: cargo test -p draconic-conformance --test host_tcp
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=7f4ff4cbef4497b8 bytes=3198 at=2026-09-04T16:56:52.726Z

## Pool

Durable links to task-pool ids. Never drop them.

- `[[h06-workspace-timeout]]`

## See also

ROADMAP.md H06, `tests/conformance/tests/host_tcp.rs`, `tests/conformance/fixtures/host/net/tcp`, `crates/draconic-backend-llvm/src/host_tcp.rs`, `crates/draconic-runtime`, docs/adr/0008-host-io-sockets-first-http.md, CONTEXT.md, `.heio/planning/locations/host-io.md`, [[s-h06]], [[ticket-152-h06-workspace-timeout]].
