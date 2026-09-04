---
id: "s-h04-workspace-timeout"
title: "H04 workspace tests finish"
kind: slice
status: met
sprint: "platform"
tags: []
created_at: "2026-09-04T16:20:21Z"
updated_at: "2026-09-04T16:54:49Z"
claimed-by: c3a09c4b-ffd9-41c2-8957-744c5232224d
---

# H04 workspace tests finish

## Why

Review of [[s-h04]] left ROADMAP H04 unfinished: O1 (`host_fs`) held, but O2 `cargo test --workspace` timed out at 120s. The host-io location still needs the H04 Loop to leave the workspace green, not only the host fs conformance harness.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP H04 work. The host fs conformance harness stays green. If the hang comes from the H04 change, fix that filesystem read / write / dirs surface so both checks hold. Mark H04 `done` only when those tests are green.

## Blocked by

None.

## Non-goals

- **Re-opening [[s-h04]]**: that slice stays sealed `failed`
- **H04.01**: File read: whole-file bytes + UTF-8 text; missing → typed error (already `done`)
- **H04.02**: File write / append; create/truncate (already `done`)
- **H04.03**: `exists` / `stat` (already `done`)
- **H04.04**: Directory mkdir / readdir / rmdir / remove file (already `done`)
- **H04.05**: Rename / copy / delete file (already `done`)
- **H04.06**: Open handle open/read/write/seek/close (already `done`)
- **H03**: path helpers (string ops; no I/O)
- **H00**: host I/O surface policy
- **R02**: permission grant/deny for fs

## Oracle checklist

- [x] O1: workspace tests finish after the H04 Loop
  CHECK: cargo test --workspace --offline --lib --bins && cargo test -p draconic-conformance --test host_fs --offline && cargo test -p draconic-runtime --lib --offline
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=9fd8abebcd4499b0 bytes=102860 at=2026-09-04T16:54:15.765Z

- [x] O2: H04 filesystem read / write / dirs stay locked by the host fs conformance tests
  CHECK: cargo test -p draconic-conformance --test host_fs
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=a7c43d77322083cd bytes=3798 at=2026-09-04T16:54:18.642Z

## Pool

Durable links to task-pool ids. Never drop them.

- `[[h04-workspace-timeout]]`

## See also

ROADMAP.md H04, `tests/conformance/tests/host_fs.rs`, `tests/conformance/fixtures/host/fs`, `crates/draconic-backend-llvm/src/host_fs.rs`, docs/adr/0008-host-io-sockets-first-http.md, CONTEXT.md, `.heio/planning/locations/host-io.md`, [[s-h04]], [[ticket-150-h04-workspace-timeout]].
