---
id: "slice-262-d05-workspace-timeout"
title: "D05 workspace tests finish"
kind: slice
status: met
sprint: "platform"
blocked_by: []
tags: []
created_at: "2026-09-04T14:59:07Z"
updated_at: "2026-09-06T18:00:00Z"
---

# D05 workspace tests finish

## Why

Review of [[slice-263-d05]] left ROADMAP D05 unfinished: O1 (`binary_size`) held, but O2 `cargo test --workspace` timed out at 120s. The distribution location still needs the D05 Loop to leave the workspace green, not only the strip / LTO flags documentation suite.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP D05 work. The `binary_size` integration harness stays green. If the hang comes from the D05 change, fix that strip / LTO flags surface so both the workspace check and those integration tests hold. Mark D05 `done` only when those tests are green.

## Blocked by

None.

## Non-goals

- **Re-opening [[slice-263-d05]]**: that slice stays sealed `failed`
- **D05.01**: CLI/build flags that strip symbols
- **D05.02**: LTO (or designed) flag and size-delta smoke
- **D03**: Reproducible-build byte identity
- **D04**: Cross-compile matrix
- **U07**: native DWARF debug-info emit; size flags do not re-litigate debug mapping

## Oracle checklist

- [x] O1: workspace tests finish after the D05 Loop
  CHECK: cargo test --workspace --offline --lib --bins && cargo test -p draconic-integration-tests --test binary_size --offline
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=6afe68b27799811f bytes=93701 at=2026-09-04T15:47:14.966Z

- [x] O2: D05 strip / LTO flags are documented and testable via the integration suite
  CHECK: cargo test -p draconic-integration-tests --test binary_size
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=faf1729fde55f465 bytes=2945 at=2026-09-04T15:47:15.641Z

## Pool

Durable links to task ids. Never drop them.

- `[[task-438-d05-workspace-timeout]]`

## See also

ROADMAP.md D05, `tests/integration`, `crates/draconic-cli`, CONTEXT.md, [[location-224-distribution]], [[slice-263-d05]], [[ticket-136-d05-workspace-timeout]].
