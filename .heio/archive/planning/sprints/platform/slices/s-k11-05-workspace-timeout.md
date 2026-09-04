---
id: "s-k11-05-workspace-timeout"
title: "K11.05 workspace tests finish"
kind: slice
status: met
sprint: "platform"
tags: []
created_at: "2026-09-04T19:32:25Z"
updated_at: "2026-09-04T20:07:02Z"
claimed-by: 0afbf59e-232f-4b78-9f39-5dc1d922f55b
---

# K11.05 workspace tests finish

## Why

Review of [[s-k11-05]] left ROADMAP K11.05 unfinished: O1 (`draconic-pkg` yank) held, but O2 `cargo test --workspace` timed out at 120s. The packages location still needs the K11.05 Loop to leave the workspace green, not only the yank/retract crate tests.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP K11.05 work. The `draconic-pkg` yank tests stay green. If the hang comes from the K11.05 change, fix that yank/retract surface so both the workspace check and those crate tests hold. Mark K11.05 `done` only when those tests are green.

## Blocked by

None.

## Non-goals

- **Re-opening [[s-k11-05]]**: that slice stays sealed `failed`
- **K11**: Post-v1 packaging umbrella (not this child)
- **K11.01**: Private git auth (HTTPS token / SSH)
- **K11.02**: `replace` directive: fork/local override
- **K11.03**: Multi-module monorepo (subdir module paths)
- **K11.04**: Module proxy/mirror (git still canonical)
- **K08**: lock-hash integrity / tampered cache (already a v1 bar)
- npm unpublish or crates.io yank as the primary shape

## Oracle checklist

- [x] O1: workspace tests finish after the K11.05 Loop
  CHECK: cargo test --workspace --offline --lib --bins && cargo test -p draconic-pkg --offline yank
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=bfa696ed037d028a bytes=92726 at=2026-09-04T20:06:45.569Z

- [x] O2: K11.05 yank/retract stays locked by the draconic-pkg yank tests
  CHECK: cargo test -p draconic-pkg yank
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=b50d3ad05f421a83 bytes=1157 at=2026-09-04T20:06:45.866Z

## Pool

Durable links to task-pool ids. Never drop them.

- `[[k11-05-workspace-timeout]]`

## See also

ROADMAP.md K11.05, `crates/draconic-pkg`, docs/adr/0009-go-style-git-packages.md, CONTEXT.md, `.heio/planning/locations/packages.md`, [[s-k11-05]], [[ticket-177-k11-05-workspace-timeout]].
