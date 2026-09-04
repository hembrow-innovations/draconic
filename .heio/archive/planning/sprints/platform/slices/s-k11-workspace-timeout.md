---
id: "s-k11-workspace-timeout"
title: "K11 workspace tests finish"
kind: slice
status: met
sprint: "platform"
tags: []
created_at: "2026-09-04T19:37:00Z"
updated_at: "2026-09-04T20:09:17Z"
claimed-by: dfd94a41-7138-4675-b840-c511007814da
---

# K11 workspace tests finish

## Why

Review of [[s-k11]] left ROADMAP K11 unfinished: O1 (`draconic-pkg` k11) held, but O2 `cargo test --workspace` timed out at 120s. The packages location still needs the K11 Loop to leave the workspace green, not only the post-v1 packaging crate tests.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP K11 work. The `draconic-pkg` k11 tests stay green. If the hang comes from the K11 change, fix that later-not-v1 packaging surface so both the workspace check and those crate tests hold. Mark K11 `done` only when those tests are green.

## Blocked by

None.

## Non-goals

- **Re-opening [[s-k11]]**: that slice stays sealed `failed`
- **K11.01**: Private git auth (HTTPS token / SSH)
- **K11.02**: `replace` directive: fork/local override
- **K11.03**: Multi-module monorepo (subdir module paths)
- **K11.04**: Module proxy/mirror (git still canonical)
- **K11.05**: Yank/retract when advisory source configured
- **K01–K10**: Manifest, lock, cache, resolve, CLI, build integration, integrity, E2E, in-repo demo (v1 bar and demo)
- npm-compatible registry or crates.io clone (rejected as v1 primary in ADR-0009)

## Oracle checklist

- [x] O1: workspace tests finish after the K11 Loop
  CHECK: cargo test --workspace --offline --lib --bins && cargo test -p draconic-pkg --offline k11
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=c7f8b224ecf934e1 bytes=94905 at=2026-09-04T20:09:03.628Z

- [x] O2: K11 post-v1 packaging stays locked by the draconic-pkg k11 tests
  CHECK: cargo test -p draconic-pkg k11
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=3ebb0507b4b4eb29 bytes=3335 at=2026-09-04T20:09:03.934Z

## Pool

Durable links to task-pool ids. Never drop them.

- `[[k11-workspace-timeout]]`

## See also

ROADMAP.md K11, `crates/draconic-pkg`, docs/adr/0009-go-style-git-packages.md, CONTEXT.md, `.heio/planning/locations/packages.md`, [[s-k11]], [[ticket-178-k11-workspace-timeout]].
