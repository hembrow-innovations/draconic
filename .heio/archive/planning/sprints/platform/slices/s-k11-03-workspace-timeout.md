---
id: "s-k11-03-workspace-timeout"
title: "K11.03 workspace tests finish"
kind: slice
status: met
sprint: "platform"
tags: []
created_at: "2026-09-04T19:16:47Z"
updated_at: "2026-09-04T19:26:57Z"
claimed-by: 45d5b021-379a-4bb0-9703-47223055ff6f
---

# K11.03 workspace tests finish

## Why

Review of [[s-k11-03]] left ROADMAP K11.03 unfinished: O1 (`draconic-pkg` subdir) held, but O2 `cargo test --workspace` timed out at 120s. The packages location still needs the K11.03 Loop to leave the workspace green, not only the multi-module monorepo subdir module path crate tests.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP K11.03 work. The `draconic-pkg` subdir module path tests stay green. If the hang comes from the K11.03 change, fix that subdir module-path surface so both the workspace check and those crate tests hold. Mark K11.03 `done` only when those tests are green.

## Blocked by

None.

## Non-goals

- **Re-opening [[s-k11-03]]**: that slice stays sealed `failed`
- **K11**: Post-v1 packaging umbrella (not this child)
- **K11.01**: Private git auth (HTTPS token / SSH)
- **K11.02**: `replace` directive: fork/local override
- **K11.04**: Module proxy/mirror (git still canonical)
- **K11.05**: Yank/retract when advisory source configured
- **K03**: Module cache layout / git clone at repo root (already landed)
- **K06**: Import resolve / package boundary for a single-module checkout

## Oracle checklist

- [x] O1: workspace tests finish after the K11.03 Loop
  CHECK: cargo test --workspace --offline --lib --bins && cargo test -p draconic-pkg --offline subdir
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=f1bdf9312748bb2d bytes=92355 at=2026-09-04T19:26:36.164Z

- [x] O2: K11.03 subdir module paths stay locked by the draconic-pkg subdir tests
  CHECK: cargo test -p draconic-pkg subdir
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=1e97e1e741c93201 bytes=1453 at=2026-09-04T19:26:36.490Z

## Pool

Durable links to task-pool ids. Never drop them.

- `[[k11-03-workspace-timeout]]`

## See also

ROADMAP.md K11.03, `crates/draconic-pkg`, docs/adr/0009-go-style-git-packages.md, CONTEXT.md, `.heio/planning/locations/packages.md`, [[s-k11-03]], [[ticket-175-k11-03-workspace-timeout]].
