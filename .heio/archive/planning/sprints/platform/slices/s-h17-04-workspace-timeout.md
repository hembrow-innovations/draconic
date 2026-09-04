---
id: "s-h17-04-workspace-timeout"
title: "H17.04 workspace tests finish"
kind: slice
status: met
sprint: "platform"
tags: []
created_at: "2026-09-04T17:54:04Z"
updated_at: "2026-09-04T18:19:38Z"
claimed-by: bafbf7eb-d9cd-4b69-8a58-cbf9cfb7abc6
---

# H17.04 workspace tests finish

## Why

Review of [[s-h17-04]] left ROADMAP H17.04 unfinished: O1 (`host_policy`) held, but O2 `cargo test --workspace` timed out at 120s. The host-io location still needs the H17.04 Loop to leave the workspace green, not only the host policy conformance harness for the optional JS/Node bridge.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP H17.04 work. The host policy conformance harness stays green. If the hang comes from the H17.04 change, fix that optional JS/Node bridge subset so both checks hold. Mark H17.04 `done` only when those tests are green.

## Blocked by

None.

## Non-goals

- **Re-opening [[s-h17-04]]**: that slice stays sealed `failed`
- **H17.01**: `examples/http-echo` pure Draconic native HTTP/1.1 (already `done`)
- **H17.02**: integration start echo, client request, assert status/body, shutdown (already `done`)
- **H17.03**: `examples/todo` C host cutover → Draconic native serve (already `done`)
- **H17 parent remainder**: Success Programs & host cutover as one umbrella row
- **H00**: host I/O surface policy parent row
- **H06.06**: TCP listen/accept hard-error on js (already `done` as the until-bridge lock)
- **H09.03**: DNS on js hard-error (already `done` as the until-bridge lock)
- **H10.07**: HTTP listen helpers hard-error on js (already `done` as the until-bridge lock)
- **P04**: flagship service example
- Full Node-shaped `http` / `net` / `dgram` modules
- TLS, HTTP/2, or WebSocket on js

## Oracle checklist

- [x] O1: workspace tests finish after the H17.04 Loop
  CHECK: cargo test --workspace --offline --lib --bins && cargo test -p draconic-conformance --test host_policy --offline && cargo test -p draconic-runtime --lib --offline
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=d381558f3417d74e bytes=103659 at=2026-09-04T18:19:21.325Z

- [x] O2: H17.04 optional JS/Node bridge subset stays locked by the host policy conformance tests
  CHECK: cargo test -p draconic-conformance --test host_policy
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=c703ec78e0ea6587 bytes=4397 at=2026-09-04T18:19:23.066Z

## Pool

Durable links to task-pool ids. Never drop them.

- `[[h17-04-workspace-timeout]]`

## See also

ROADMAP.md H17.04, ROADMAP.md H06.06, ROADMAP.md H09.03, ROADMAP.md H10.07, `tests/conformance/tests/host_policy.rs`, `tests/conformance/fixtures/host/policy`, `crates/draconic-check/src/host_api.rs`, docs/adr/0008-host-io-sockets-first-http.md, CONTEXT.md, `.heio/planning/locations/host-io.md`, [[s-h17-04]], [[ticket-163-h17-04-workspace-timeout]].
