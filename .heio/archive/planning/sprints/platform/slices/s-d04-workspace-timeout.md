---
id: "s-d04-workspace-timeout"
title: "D04 workspace tests finish"
kind: slice
status: met
sprint: "platform"
tags: []
created_at: "2026-09-04T14:35:09Z"
updated_at: "2026-09-04T14:54:10Z"
claimed-by: 413a8129-c518-4cd7-b790-b1b4c1c3f88a
---

# D04 workspace tests finish

## Why

Review of [[s-d04]] left ROADMAP D04 unfinished: O1 (`cross_compile`) held, but O2 `cargo test --workspace` timed out at 120s. The distribution location still needs the D04 Loop to leave the workspace green, not only the available-OS/arch matrix suite.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP D04 work. The `cross_compile` harness stays green. If the hang comes from the D04 change, fix that linux/darwin/windows × amd64/arm64 (as available) matrix surface so both the workspace check and those integration tests hold. Mark D04 `done` only when those tests are green.

## Blocked by

None.

## Non-goals

- **Re-opening [[s-d04]]**: that slice stays sealed `failed`
- **D04.01**: Cross-compile: at least one non-host triple smoke
- **D04.02**: Matrix docs + CI jobs for available OS/arch pairs
- **D03**: Reproducible-build byte identity
- **D05**: Strip / LTO size flags
- **D01**: Release binaries + install script

## Oracle checklist

- [x] O1: workspace tests finish after the D04 Loop
  CHECK: cargo test --workspace --offline --lib --bins && cargo test -p draconic-integration-tests --test cross_compile --offline
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=3a7851bbf78f3779 bytes=93536 at=2026-09-04T14:53:24.499Z

- [x] O2: D04 cross-compile matrix for available OS/arch pairs is locked by the integration suite
  CHECK: cargo test -p draconic-integration-tests --test cross_compile
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=1d1c436f001bfb34 bytes=3007 at=2026-09-04T14:53:24.919Z

## Pool

Durable links to task-pool ids. Never drop them.

- `[[d04-workspace-timeout]]`

## See also

ROADMAP.md D04, `tests/integration/tests/cross_compile.rs`, `crates/draconic-backend-llvm`, CONTEXT.md, [[distribution]], [[s-d04]], [[ticket-133-d04-workspace-timeout]].
