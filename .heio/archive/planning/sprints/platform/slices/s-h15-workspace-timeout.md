---
id: "s-h15-workspace-timeout"
title: "H15 workspace tests finish"
kind: slice
status: met
sprint: "platform"
tags: []
created_at: "2026-09-04T17:43:08Z"
updated_at: "2026-09-04T18:06:19Z"
claimed-by: 0c8a3d53-d523-4feb-b0c3-5b65c4031e56
---

# H15 workspace tests finish

## Why

Review of [[s-h15]] left ROADMAP H15 unfinished: O1 (`host_process`) held, but O2 `cargo test --workspace` timed out at 120s. The host-io location still needs the H15 Loop to leave the workspace green, not only the host process subprocess conformance harness.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP H15 work. The host process subprocess conformance harness stays green. If the hang comes from the H15 change, fix that spawn/run, capture/kill, and native async wait surface so both checks hold. Mark H15 `done` only when those tests are green.

## Blocked by

None.

## Non-goals

- **Re-opening [[s-h15]]**: that slice stays sealed `failed`
- **H15.01**: spawn/run argv, env subset, cwd, wait exit code (already `done`)
- **H15.02**: capture stdout/stderr; write stdin; kill child (already `done`)
- **H15.03**: async subprocess exit via job queue / Promise (already `done`)
- **H01**: process args, env, exit
- **H14**: signals
- **H16**: OS misc
- **H00**: host I/O surface policy
- js subprocess APIs or a Node polyfill (native-only until an explicit bridge row)

## Oracle checklist

- [x] O1: workspace tests finish after the H15 Loop
  CHECK: cargo test --workspace --offline --lib --bins && cargo test -p draconic-conformance --test host_process --offline && cargo test -p draconic-runtime --lib --offline
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=c475a8e649231d81 bytes=103842 at=2026-09-04T18:06:15.236Z

- [x] O2: H15 subprocess spawn/run, capture/kill, and native async wait stay locked by the host process conformance tests
  CHECK: cargo test -p draconic-conformance --test host_process
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=a7af9c8afa6de8c5 bytes=4579 at=2026-09-04T18:06:19.608Z

## Pool

Durable links to task-pool ids. Never drop them.

- `[[h15-workspace-timeout]]`

## See also

ROADMAP.md H15, `tests/conformance/tests/host_process.rs`, `tests/conformance/fixtures/host/process`, `crates/draconic-backend-llvm/src/host_subprocess.rs`, `crates/draconic-runtime`, docs/adr/0008-host-io-sockets-first-http.md, CONTEXT.md, `.heio/planning/locations/host-io.md`, [[s-h15]], [[ticket-161-h15-workspace-timeout]].
