---
id: "s-d03-02-workspace-tests"
title: "D03.02 workspace tests pass"
kind: slice
status: met
sprint: "platform"
tags: []
created_at: "2026-09-04T14:20:17Z"
updated_at: "2026-09-04T14:37:03Z"
claimed-by: 913d9d67-9681-48bb-9286-56bb3385253a
---

# D03.02 workspace tests pass

## Why

Review of [[s-d03-02]] left ROADMAP D03.02 unfinished: O1 (`reproducible_emit`) held, but O2 `cargo test --workspace` failed (exit 101). Workspace did not compile `draconic-pkg` lib tests: `LaterPackaging` undeclared in `crates/draconic-pkg/src/later.rs`. The distribution location still needs the D03.02 Loop to leave the workspace green, not only the two-build emit suite.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP D03.02 work. The `reproducible_emit` harness stays green. If the compile failure comes from the D03.02 change, fix that same-source-plus-pin emit surface so both the workspace check and those integration tests hold. If it is `draconic-pkg` `later.rs` (`LaterPackaging` undeclared under lib tests), make that crate compile under workspace tests so both hold. Mark D03.02 `done` only when those tests are green.

## Blocked by

None.

## Non-goals

- **Re-opening [[s-d03-02]]**: that slice stays sealed `failed`
- **D03.01**: Document reproducibility expectations (timestamps, paths)
- **D03 parent remainder**: combining docs + emit identity as one umbrella row
- **D02**: Toolchain version pin in `draconic.toml`
- **D01**: Release binaries + install script
- **D04**: Cross-compile matrix
- **D05**: Strip / LTO size flags

## Oracle checklist

- [x] O1: workspace tests pass after the D03.02 Loop
  CHECK: cargo test --workspace --offline --lib --bins && cargo test -p draconic-integration-tests --test reproducible_emit --offline
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=eae8ba510b619d69 bytes=93581 at=2026-09-04T14:36:42.965Z

- [x] O2: D03.02 same source + pin emit is byte-identical or documented-equivalent, locked by the integration suite
  CHECK: cargo test -p draconic-integration-tests --test reproducible_emit
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=2a5ca030e2e4bbbb bytes=3052 at=2026-09-04T14:36:43.013Z

## Pool

Durable links to task-pool ids. Never drop them.

- `[[d03-02-workspace-tests]]`

## See also

ROADMAP.md D03.02, `tests/integration/tests/reproducible_emit.rs`, `crates/draconic-pkg/src/later.rs`, CONTEXT.md, [[distribution]], [[s-d03-02]], [[ticket-129-d03-02-workspace-tests]].
