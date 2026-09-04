---
id: "s-k11-04-workspace-timeout"
title: "K11.04 workspace tests finish"
kind: slice
status: met
sprint: "platform"
tags: []
created_at: "2026-09-04T19:24:33Z"
updated_at: "2026-09-04T20:05:23Z"
claimed-by: 7cda2eb8-d157-4784-be49-f9223dcd3f92
---

# K11.04 workspace tests finish

## Why

Review of [[s-k11-04]] left ROADMAP K11.04 unfinished: O1 (`draconic-pkg` k11_04) held, but O2 `cargo test --workspace` timed out at 120s. The packages location still needs the K11.04 Loop to leave the workspace green, not only the module proxy/mirror crate tests.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP K11.04 work. The `draconic-pkg` k11_04 module proxy/mirror tests stay green. If the hang comes from the K11.04 change, fix that proxy/mirror surface so both the workspace check and those crate tests hold. Mark K11.04 `done` only when those tests are green.

## Blocked by

None.

## Non-goals

- **Re-opening [[s-k11-04]]**: that slice stays sealed `failed`
- **K11**: Post-v1 packaging umbrella (not this child)
- **K11.01**: Private git auth (HTTPS token / SSH)
- **K11.02**: `replace` directive: fork/local override
- **K11.03**: Multi-module monorepo (subdir module paths)
- **K11.05**: Yank/retract when advisory source configured
- **K03**: Module cache layout / git clone at repo root (already landed)
- a central npm/crates.io registry as canonical source (rejected as v1 primary in ADR-0009)

## Oracle checklist

- [x] O1: workspace tests finish after the K11.04 Loop
  CHECK: cargo test --workspace --offline --lib --bins && cargo test -p draconic-pkg --offline k11_04
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=f254937063548fb3 bytes=92883 at=2026-09-04T20:05:06.327Z

- [x] O2: K11.04 module proxy/mirror stays locked by the draconic-pkg k11_04 tests
  CHECK: cargo test -p draconic-pkg k11_04
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=77f4d6885b548161 bytes=1313 at=2026-09-04T20:05:06.577Z

## Pool

Durable links to task-pool ids. Never drop them.

- `[[k11-04-workspace-timeout]]`

## See also

ROADMAP.md K11.04, `crates/draconic-pkg`, docs/adr/0009-go-style-git-packages.md, CONTEXT.md, `.heio/planning/locations/packages.md`, [[s-k11-04]], [[ticket-176-k11-04-workspace-timeout]].
