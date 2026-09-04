---
id: "s-h14-workspace-timeout"
title: "H14 workspace tests finish"
kind: slice
status: met
sprint: "platform"
tags: []
created_at: "2026-09-04T17:34:55Z"
updated_at: "2026-09-04T18:04:00Z"
claimed-by: ac10efff-c9d5-4303-a203-255dc5d1239f
---

# H14 workspace tests finish

## Why

Review of [[s-h14]] left ROADMAP H14 unfinished: O1 (`host_process signal`) and O2 (`host_signal`) held, but O3 `cargo test --workspace` timed out at 120s. The host-io location still needs the H14 Loop to leave the workspace green, not only the host process signal conformance harness and Runtime signal ABI tests.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP H14 work. The host process signal conformance harness and the runtime crate signal tests stay green. If the hang comes from the H14 change, fix that SIGINT/SIGTERM watch, ignore, and restore-default surface so both checks hold. Mark H14 `done` only when those tests are green.

## Blocked by

None.

## Non-goals

- **Re-opening [[s-h14]]**: that slice stays sealed `failed`
- **H14.01**: Signal watch SIGINT/SIGTERM → handler/job; default terminate documented (already `done`)
- **H14.02**: Signal ignore / restore default (already `done`)
- **H01**: process args, env, exit
- **H15**: subprocess
- **H16**: OS misc
- **H00**: host I/O surface policy
- js signal APIs or a Node polyfill (native-only until an explicit bridge row)

## Oracle checklist

- [x] O1: workspace tests finish after the H14 Loop
  CHECK: cargo test --workspace --offline --lib --bins && cargo test -p draconic-conformance --test host_process --offline signal && cargo test -p draconic-runtime --lib --offline
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=1eafd3aaa40d0fed bytes=102492 at=2026-09-04T18:03:36.720Z

- [x] O2: H14 signal watch, ignore, and restore stay locked by the host process signal conformance tests
  CHECK: cargo test -p draconic-conformance --test host_process signal
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=ecac8c66a99e8028 bytes=3230 at=2026-09-04T18:03:37.693Z

- [x] O3: H14 Runtime signal ABI (watch/raise/ignore/restore) stays locked by the runtime crate tests
  CHECK: cargo test -p draconic-runtime host_signal
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=d4906294a8617d54 bytes=543 at=2026-09-04T18:03:38.387Z

## Pool

Durable links to task-pool ids. Never drop them.

- `[[h14-workspace-timeout]]`

## See also

ROADMAP.md H14, `tests/conformance/tests/host_process.rs`, `tests/conformance/fixtures/host/process`, `crates/draconic-backend-llvm/src/host_signals.rs`, `crates/draconic-runtime`, docs/adr/0008-host-io-sockets-first-http.md, CONTEXT.md, `.heio/planning/locations/host-io.md`, [[s-h14]], [[ticket-160-h14-workspace-timeout]].
