---
id: "s-d05-02-workspace-timeout"
title: "D05.02 workspace tests finish"
kind: slice
status: met
sprint: "platform"
tags: []
created_at: "2026-09-04T14:48:54Z"
updated_at: "2026-09-04T14:59:01Z"
claimed-by: 76565cde-a87e-45cd-a201-a751525c214d
---

# D05.02 workspace tests finish

## Why

Review of [[s-d05-02]] left ROADMAP D05.02 unfinished: O1 (`lto_flag`) and O2 (`binary_size_lto`) held, but O3 `cargo test --workspace` timed out at 120s. The distribution location still needs the D05.02 Loop to leave the workspace green, not only the CLI/build LTO flag and size-delta suite.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP D05.02 work. The `lto_flag` CLI tests and `binary_size_lto` integration harness stay green. If the hang comes from the D05.02 change, fix that CLI/build LTO surface so both the workspace check and those tests hold. Mark D05.02 `done` only when those tests are green.

## Blocked by

None.

## Non-goals

- **Re-opening [[s-d05-02]]**: that slice stays sealed `failed`
- **D05.01**: CLI/build flags that strip symbols
- **D05 parent remainder**: documenting strip and LTO together as one umbrella row
- **D03**: reproducible-build byte identity
- **D04**: Cross-compile matrix
- **U07**: native DWARF debug-info emit; LTO does not re-litigate debug mapping

## Oracle checklist

- [x] O1: workspace tests finish after the D05.02 Loop
  CHECK: cargo test --workspace --offline --lib --bins && cargo test -p draconic-cli --test lto_flag --offline && cargo test -p draconic-integration-tests --test binary_size_lto --offline
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=5568a6aea017bb8a bytes=96298 at=2026-09-04T14:58:45.737Z

- [x] O2: D05.02 LTO (or designed) flag is locked by the CLI crate tests
  CHECK: cargo test -p draconic-cli --test lto_flag
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=227e8e42b6df29a5 bytes=2915 at=2026-09-04T14:58:46.630Z

- [x] O3: D05.02 LTO size-delta smoke is locked by the integration suite
  CHECK: cargo test -p draconic-integration-tests --test binary_size_lto
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=0e4e74e319681d7e bytes=2854 at=2026-09-04T14:58:47.381Z

## Pool

Durable links to task-pool ids. Never drop them.

- `[[d05-02-workspace-timeout]]`

## See also

ROADMAP.md D05.02, `crates/draconic-cli`, `tests/integration`, CONTEXT.md, [[distribution]], [[s-d05-02]], [[ticket-135-d05-02-workspace-timeout]].
