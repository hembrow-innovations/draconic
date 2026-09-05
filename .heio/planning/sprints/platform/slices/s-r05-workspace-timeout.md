---
id: "s-r05-workspace-timeout"
title: "R05 workspace tests finish"
kind: slice
status: released
sprint: "platform"
tags: []
created_at: "2026-09-05T20:10:00Z"
updated_at: "2026-09-05T20:29:12Z"
---

# R05 workspace tests finish

## Why

Review of [[s-r05]] left O2 unmet: workspace CHECK budget blew after EXPECT matched. R05 / R05.01 stay `done`. Beat the budget, not a new language atom.

## Done

`cargo test --workspace` exits 0 and prints `test result: ok.` inside the oracle CHECK budget. parser fuzz stays green.

## Blocked by

None.

## Non-goals

- **Re-opening [[s-r05]]**: sealed `failed`
- **Minting a new R05 Loop atom**

## Oracle checklist

- [ ] O1: R05 parser fuzz stays locked
  CHECK: cargo test -p draconic-parser --lib fuzz
  EXPECT: test result: ok.
  EVIDENCE: pending

- [ ] O2: workspace tests finish after the R05 Loop
  CHECK: cargo test --workspace
  EXPECT: test result: ok.
  EVIDENCE: pending

## Pool

- `[[r05-workspace-timeout]]`

## See also

ROADMAP.md R05, [[ticket-197-r05-workspace-timeout]], [[s-r05]].
