---
id: "s-l10-02-workspace-timeout"
title: "L10.02 workspace tests finish"
kind: slice
status: failed
sprint: "platform"
tags: []
created_at: "2026-09-05T20:10:00Z"
updated_at: "2026-09-05T21:15:00Z"
---

# L10.02 workspace tests finish

## Why

Review of [[s-l10-02]] left O2 unmet: workspace CHECK budget blew after EXPECT matched. L10.02 stays `done`. Beat the budget, not a new language atom.

## Done

`cargo test --workspace` exits 0 and prints `test result: ok.` inside the oracle CHECK budget. stdlib_crypto stays green.

## Blocked by

None.

## Non-goals

- **Re-opening [[s-l10-02]]**: sealed `failed`
- **Minting a new L10.02 Loop atom**

## Oracle checklist

- [x] O1: L10.02 AEAD fixtures stay locked
  CHECK: cargo test -p draconic-conformance --test stdlib_crypto
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=5045b7fe533348f2 bytes=3470

- [ ] O2: workspace tests finish after the L10.02 Loop
  CHECK: cargo test --workspace
  EXPECT: test result: ok.
  EVIDENCE: unmet exit=101 match=yes bytes=42064
  ABANDON: leftover after --reverify; CHECK failed (exit=101 match=yes bytes=42064); cargo test --workspace exited 101 → [[ticket-207-l10-02-workspace-tests]]

## Pool

- `[[l10-02-workspace-timeout]]`

## See also

ROADMAP.md L10.02, [[ticket-193-l10-02-workspace-timeout]], [[s-l10-02]].
