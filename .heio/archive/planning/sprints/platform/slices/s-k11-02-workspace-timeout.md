---
id: "s-k11-02-workspace-timeout"
title: "K11.02 workspace tests finish"
kind: slice
status: met
sprint: "platform"
tags: []
created_at: "2026-09-04T19:08:46Z"
updated_at: "2026-09-04T19:25:13Z"
claimed-by: c14dc3ed-4f97-476a-8b07-16e5d63b03f4
---

# K11.02 workspace tests finish

## Why

Review of [[s-k11-02]] left ROADMAP K11.02 unfinished: O1 (`draconic-pkg` replace) held, but O2 `cargo test --workspace` timed out at 120s. The packages location still needs the K11.02 Loop to leave the workspace green, not only the `replace` directive fork/local override crate tests.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP K11.02 work. The `draconic-pkg` replace fork and local override tests stay green. If the hang comes from the K11.02 change, fix that `replace` directive surface so both the workspace check and those crate tests hold. Mark K11.02 `done` only when those tests are green.

## Blocked by

None.

## Non-goals

- **Re-opening [[s-k11-02]]**: that slice stays sealed `failed`
- **K11**: Post-v1 packaging umbrella (not this child)
- **K11.01**: Private git auth (HTTPS token / SSH)
- **K11.03**: Multi-module monorepo (subdir module paths)
- **K11.04**: Module proxy/mirror (git still canonical)
- **K11.05**: Yank/retract when advisory source configured
- **K01**: Manifest parse/write/url-map (already landed)

## Oracle checklist

- [x] O1: workspace tests finish after the K11.02 Loop
  CHECK: cargo test --workspace --offline --lib --bins && cargo test -p draconic-pkg --offline replace
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=60133dc21280ef77 bytes=92555 at=2026-09-04T19:25:13.316Z

- [x] O2: K11.02 `replace` fork and local override stay locked by the draconic-pkg replace tests
  CHECK: cargo test -p draconic-pkg replace
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=263689fa40f08495 bytes=1653 at=2026-09-04T19:25:13.649Z

## Pool

Durable links to task-pool ids. Never drop them.

- `[[k11-02-workspace-timeout]]`

## See also

ROADMAP.md K11.02, `crates/draconic-pkg`, docs/adr/0009-go-style-git-packages.md, CONTEXT.md, `.heio/planning/locations/packages.md`, [[s-k11-02]], [[ticket-174-k11-02-workspace-timeout]].
