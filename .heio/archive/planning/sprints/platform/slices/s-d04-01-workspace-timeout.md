---
id: "s-d04-01-workspace-timeout"
title: "D04.01 workspace tests finish"
kind: slice
status: met
sprint: "platform"
tags: []
created_at: "2026-09-04T14:27:11Z"
updated_at: "2026-09-04T14:50:03Z"
claimed-by: 54547085-1ad6-41cf-8491-2675d01b8f96
---

# D04.01 workspace tests finish

## Why

Review of [[s-d04-01]] left ROADMAP D04.01 unfinished: O1 (`cross_compile_non_host`) held, but O2 `cargo test --workspace` timed out at 120s. The distribution location still needs the D04.01 Loop to leave the workspace green, not only the non-host triple LLVM smoke.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP D04.01 work. The `cross_compile_non_host` harness stays green. If the hang comes from the D04.01 change, fix that non-host triple LLVM smoke surface so both the workspace check and those integration tests hold. Mark D04.01 `done` only when those tests are green.

## Blocked by

None.

## Non-goals

- **Re-opening [[s-d04-01]]**: that slice stays sealed `failed`
- **D04 parent remainder**: available-OS/arch matrix surface without a non-host success
- **D04.02**: Matrix docs + CI jobs for available OS/arch pairs
- **D03**: Reproducible-build byte identity
- **D05**: Strip / LTO size flags
- **D01**: Release binaries + install script

## Oracle checklist

- [x] O1: workspace tests finish after the D04.01 Loop
  CHECK: cargo test --workspace --offline --lib --bins && cargo test -p draconic-integration-tests --test cross_compile_non_host --offline
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=0e5df3c389e81234 bytes=93411 at=2026-09-04T14:49:49.654Z

- [x] O2: D04.01 non-host triple LLVM smoke is locked by the integration suite
  CHECK: cargo test -p draconic-integration-tests --test cross_compile_non_host
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=d73fbda90d784554 bytes=2882 at=2026-09-04T14:49:49.716Z

## Pool

Durable links to task-pool ids. Never drop them.

- `[[d04-01-workspace-timeout]]`

## See also

ROADMAP.md D04.01, `tests/integration/tests/cross_compile_non_host.rs`, `crates/draconic-backend-llvm`, CONTEXT.md, [[distribution]], [[s-d04-01]], [[ticket-131-d04-01-workspace-timeout]].
