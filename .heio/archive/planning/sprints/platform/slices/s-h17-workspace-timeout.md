---
id: "s-h17-workspace-timeout"
title: "H17 workspace tests finish"
kind: slice
status: met
sprint: "platform"
tags: []
created_at: "2026-09-04T18:02:48Z"
updated_at: "2026-09-04T18:21:41Z"
claimed-by: e265aecb-8c77-4fad-9394-f707660f5ad3
---

# H17 workspace tests finish

## Why

Review of [[s-h17]] left ROADMAP H17 unfinished: O1 (`http_echo` / `host_cutover`) and O2 (`todo_server`) held, but O3 `cargo test --workspace` timed out at 120s. The host-io location still needs the H17 Loop to leave the workspace green, not only the Success Programs integration harness.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP H17 work. The http-echo and todo native-serve integration tests stay green. If the hang comes from the H17 change, fix that Success Programs & host cutover so both checks hold. Mark H17 `done` only when those tests are green.

## Blocked by

None.

## Non-goals

- **Re-opening [[s-h17]]**: that slice stays sealed `failed`
- **H17.01**: `examples/http-echo` pure Draconic native HTTP/1.1 (already `done`)
- **H17.02**: integration start echo, client request, assert status/body, shutdown (already `done`)
- **H17.03**: `examples/todo` C host cutover → Draconic native serve (already `done`)
- **H17.04**: optional JS/Node bridge for subset host APIs (separate timeout retry)
- **H10**: HTTP/1.1 thin helpers
- **P04**: flagship service example (typed HTTP + fs/config + git dep)
- **P01**: fizzbuzz flagship Program

## Oracle checklist

- [x] O1: workspace tests finish after the H17 Loop
  CHECK: cargo test --workspace --offline --lib --bins && cargo test -p draconic-integration-tests --test http_echo --test host_cutover --offline && cargo test -p draconic-integration-tests --test todo_server --offline
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=b72f2eec32ed303b bytes=96841 at=2026-09-04T18:21:19.705Z

- [x] O2: H17 http-echo success program stays locked by the integration test
  CHECK: cargo test -p draconic-integration-tests --test http_echo --test host_cutover
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=23b17807646d6941 bytes=3097 at=2026-09-04T18:21:20.124Z

- [x] O3: H17 todo native serve cutover stays locked by the integration test
  CHECK: cargo test -p draconic-integration-tests --test todo_server
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=e586caed5c0e4e68 bytes=2842 at=2026-09-04T18:21:20.544Z

## Pool

Durable links to task-pool ids. Never drop them.

- `[[h17-workspace-timeout]]`

## See also

ROADMAP.md H17, `examples/http-echo`, `examples/todo`, `tests/integration/tests/http_echo.rs`, `tests/integration/tests/todo_server.rs`, docs/adr/0008-host-io-sockets-first-http.md, CONTEXT.md, `.heio/planning/locations/host-io.md`, [[s-h17]], [[ticket-164-h17-workspace-timeout]].
