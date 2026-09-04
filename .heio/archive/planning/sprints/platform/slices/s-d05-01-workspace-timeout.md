---
id: "s-d05-01-workspace-timeout"
title: "D05.01 workspace tests finish"
kind: slice
status: met
sprint: "platform"
tags: []
created_at: "2026-09-04T14:42:39Z"
updated_at: "2026-09-04T15:00:59Z"
claimed-by: 2c2e4039-0fdd-4b4f-aa77-38f281d1d843
---

# D05.01 workspace tests finish

## Why

Review of [[s-d05-01]] left ROADMAP D05.01 unfinished: O1 (`strip_symbols`) and O2 (`binary_size_strip`) held, but O3 `cargo test --workspace` timed out at 120s. The distribution location still needs the D05.01 Loop to leave the workspace green, not only the CLI/build strip-symbols suite.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP D05.01 work. The `strip_symbols` CLI tests and `binary_size_strip` integration harness stay green. If the hang comes from the D05.01 change, fix that CLI/build strip-symbols surface so both the workspace check and those tests hold. Mark D05.01 `done` only when those tests are green.

## Blocked by

None.

## Non-goals

- **Re-opening [[s-d05-01]]**: that slice stays sealed `failed`
- **D05 parent remainder**: documenting strip and LTO together as one umbrella row
- **D05.02**: LTO (or designed) flag and size-delta smoke
- **U07**: native DWARF debug-info emit; strip does not re-litigate debug mapping
- **D03**: reproducible-build byte identity
- **D04**: Cross-compile matrix

## Oracle checklist

- [x] O1: workspace tests finish after the D05.01 Loop
  CHECK: cargo test --workspace --offline --lib --bins && cargo test -p draconic-cli --test strip_symbols --offline && cargo test -p draconic-integration-tests --test binary_size_strip --offline
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=431129a498e71e29 bytes=96289 at=2026-09-04T15:00:44.779Z

- [x] O2: D05.01 CLI/build strip-symbols flag is locked by the CLI crate tests
  CHECK: cargo test -p draconic-cli --test strip_symbols
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=a4b8db4b4add38eb bytes=2893 at=2026-09-04T15:00:45.575Z

- [x] O3: D05.01 stripped native artifacts are locked by the integration suite
  CHECK: cargo test -p draconic-integration-tests --test binary_size_strip
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=19e65f9a8e2abdd3 bytes=2867 at=2026-09-04T15:00:46.270Z

## Pool

Durable links to task-pool ids. Never drop them.

- `[[d05-01-workspace-timeout]]`

## See also

ROADMAP.md D05.01, `crates/draconic-cli`, `tests/integration`, CONTEXT.md, [[distribution]], [[s-d05-01]], [[ticket-134-d05-01-workspace-timeout]].
