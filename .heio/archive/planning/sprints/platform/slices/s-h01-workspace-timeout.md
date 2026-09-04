---
id: "s-h01-workspace-timeout"
title: "H01 workspace tests finish"
kind: slice
status: met
sprint: "platform"
tags: []
created_at: "2026-09-04T15:53:06Z"
updated_at: "2026-09-04T16:38:38Z"
claimed-by: 4f6d7778-55e1-441e-a4fb-a7adc50887c9
---

# H01 workspace tests finish

## Why

Review of [[s-h01]] left ROADMAP H01 unfinished: O1 (`host_process`) held, but O2 `cargo test --workspace` timed out at 120s. The host-io location still needs the H01 Loop to leave the workspace green, not only the host process conformance harness.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP H01 work. The host process conformance harness stays green. If the hang comes from the H01 change, fix that process args/env/exit surface so both checks hold. Mark H01 `done` only when those tests are green.

## Blocked by

None.

## Non-goals

- **Re-opening [[s-h01]]**: that slice stays sealed `failed`
- **H01.01**: program args as string array (already `done`)
- **H01.02**: env get/set/delete (already `done`)
- **H01.03**: `exit(code)` / exitCode (already `done`)
- **H01.04**: `pid` + `ppid` (already `done`)
- **H00**: host I/O surface policy
- **H02**: stdio
- **H14**: signals
- **H15**: subprocess spawn/run/capture

## Oracle checklist

- [x] O1: workspace tests finish after the H01 Loop
  CHECK: cargo test --workspace --offline --lib --bins && cargo test -p draconic-conformance --test host_process --offline && cargo test -p draconic-runtime --lib --offline
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=35e6c5f5a2088933 bytes=103641 at=2026-09-04T16:38:14.790Z

- [x] O2: H01 process args, env, and exit stay locked by the host process conformance tests
  CHECK: cargo test -p draconic-conformance --test host_process
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=ab8492385fbda983 bytes=4579 at=2026-09-04T16:38:19.348Z

## Pool

Durable links to task-pool ids. Never drop them.

- `[[h01-workspace-timeout]]`

## See also

ROADMAP.md H01, `tests/conformance/tests/host_process.rs`, `tests/conformance/fixtures/host/process`, `crates/draconic-runtime`, docs/adr/0008-host-io-sockets-first-http.md, CONTEXT.md, `.heio/planning/locations/host-io.md`, [[s-h01]], [[ticket-147-h01-workspace-timeout]].
