---
id: "slice-290-h03-workspace-timeout"
title: "H03 workspace tests finish"
kind: slice
status: met
sprint: "platform"
blocked_by: []
tags: []
created_at: "2026-09-04T16:11:17Z"
updated_at: "2026-09-06T18:00:00Z"
---

# H03 workspace tests finish

## Why

Review of [[slice-291-h03]] left ROADMAP H03 unfinished: O1 (`host_path`) held, but O2 `cargo test --workspace` timed out at 120s. The host-io location still needs the H03 Loop to leave the workspace green, not only the host path conformance harness.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP H03 work. The host path conformance harness stays green. If the hang comes from the H03 change, fix that path-helper surface (string ops; no I/O) so both checks hold. Mark H03 `done` only when those tests are green.

## Blocked by

None.

## Non-goals

- **Re-opening [[slice-291-h03]]**: that slice stays sealed `failed`
- **H03.01**: `path.join` / `path.normalize` (already `done`)
- **H03.02**: `dirname` / `basename` / `extname` / `isAbsolute` (already `done`)
- **H03.03**: `path.resolve` relative to cwd (already `done`)
- **H04**: filesystem read / write / dirs
- **H16**: OS misc (cwd already landed as H16.01)
- **H00**: host I/O surface policy

## Oracle checklist

- [x] O1: workspace tests finish after the H03 Loop
  CHECK: cargo test --workspace --offline --lib --bins && cargo test -p draconic-conformance --test host_path --offline && cargo test -p draconic-runtime --lib --offline
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=664eddf9cec81514 bytes=102531 at=2026-09-04T16:47:25.804Z

- [x] O2: H03 path helpers stay locked by the host path conformance tests
  CHECK: cargo test -p draconic-conformance --test host_path
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=775f9880ca23f5d4 bytes=3469 at=2026-09-04T16:47:27.557Z

## Pool

Durable links to task ids. Never drop them.

- `[[task-466-h03-workspace-timeout]]`

## See also

ROADMAP.md H03, `tests/conformance/tests/host_path.rs`, `tests/conformance/fixtures/host/path`, `crates/draconic-backend-llvm/src/host_path.rs`, docs/adr/0008-host-io-sockets-first-http.md, CONTEXT.md, `.heio/planning/locations/location-219-host-io.md`, [[slice-291-h03]], [[ticket-149-h03-workspace-timeout]].
