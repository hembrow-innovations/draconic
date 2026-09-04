---
id: "s-d03-workspace-timeout"
title: "D03 workspace tests finish"
kind: slice
status: met
sprint: "platform"
tags: []
created_at: "2026-09-04T14:23:20Z"
updated_at: "2026-09-04T14:44:10Z"
claimed-by: a5ea9964-59a3-4a68-b668-0a844da8a249
---

# D03 workspace tests finish

## Why

Review of [[s-d03]] left ROADMAP D03 unfinished: O1 (`reproducible_builds`) held, but O2 `cargo test --workspace` timed out at 120s. The distribution location still needs the D03 Loop to leave the workspace green, not only the same-source-plus-pin suite.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP D03 work. The `reproducible_builds` harness stays green. If the hang comes from the D03 change, fix that same-source-plus-pin documented-equivalent artifacts surface so both the workspace check and those integration tests hold. Mark D03 `done` only when those tests are green.

## Blocked by

None.

## Non-goals

- **Re-opening [[s-d03]]**: that slice stays sealed `failed`
- **D03.01**: Document reproducibility expectations (timestamps, paths)
- **D03.02**: Same source + pin → byte-identical or documented-equivalent emit
- **D02**: Toolchain version pin in `draconic.toml`
- **D01**: Release binaries + install script
- **D04**: Cross-compile matrix and CI jobs
- **D05**: Strip / LTO size flags

## Oracle checklist

- [x] O1: workspace tests finish after the D03 Loop
  CHECK: cargo test --workspace --offline --lib --bins && cargo test -p draconic-integration-tests --test reproducible_builds --offline
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=8a0e663c32fed7f8 bytes=93492 at=2026-09-04T14:43:48.581Z

- [x] O2: D03 same source + pin yields documented-equivalent artifacts, locked by the integration suite
  CHECK: cargo test -p draconic-integration-tests --test reproducible_builds
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=caabe9b167c63768 bytes=2963 at=2026-09-04T14:43:48.628Z

## Pool

Durable links to task-pool ids. Never drop them.

- `[[d03-workspace-timeout]]`

## See also

ROADMAP.md D03, `tests/integration/tests/reproducible_builds.rs`, CONTEXT.md, [[distribution]], [[s-d03]], [[ticket-130-d03-workspace-timeout]].
