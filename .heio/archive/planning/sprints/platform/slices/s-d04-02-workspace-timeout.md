---
id: "s-d04-02-workspace-timeout"
title: "D04.02 workspace tests finish"
kind: slice
status: met
sprint: "platform"
tags: []
created_at: "2026-09-04T14:31:04Z"
updated_at: "2026-09-04T14:51:56Z"
claimed-by: b54b6126-1fc1-4ceb-9d8c-350f8ed4c332
---

# D04.02 workspace tests finish

## Why

Review of [[s-d04-02]] left ROADMAP D04.02 unfinished: O1 (`cross_compile_matrix`) held, but O2 `cargo test --workspace` timed out at 120s. The distribution location still needs the D04.02 Loop to leave the workspace green, not only the matrix docs + CI jobs suite.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP D04.02 work. The `cross_compile_matrix` harness stays green. If the hang comes from the D04.02 change, fix that matrix docs + CI jobs surface so both the workspace check and those integration tests hold. Mark D04.02 `done` only when those tests are green.

## Blocked by

None.

## Non-goals

- **Re-opening [[s-d04-02]]**: that slice stays sealed `failed`
- **D04.01**: Cross-compile: at least one non-host triple smoke
- **D04 parent remainder**: pairs that are not available stay out of this sitting
- **D05**: Strip / LTO size flags
- **D01**: Release binaries + install script
- **D03**: Reproducible-build byte identity

## Oracle checklist

- [x] O1: workspace tests finish after the D04.02 Loop
  CHECK: cargo test --workspace --offline --lib --bins && cargo test -p draconic-integration-tests --test cross_compile_matrix --offline
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=e4d91c703cdcda40 bytes=93444 at=2026-09-04T14:51:42.663Z

- [x] O2: D04.02 matrix docs and CI jobs for available OS/arch pairs are locked by the integration suite
  CHECK: cargo test -p draconic-integration-tests --test cross_compile_matrix
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=10f31a0f05ef888f bytes=2915 at=2026-09-04T14:51:42.703Z

## Pool

Durable links to task-pool ids. Never drop them.

- `[[d04-02-workspace-timeout]]`

## See also

ROADMAP.md D04.02, `tests/integration/tests/cross_compile_matrix.rs`, `.github/workflows`, CONTEXT.md, [[distribution]], [[s-d04-02]], [[ticket-132-d04-02-workspace-timeout]].
