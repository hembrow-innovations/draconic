---
id: "slice-397-r05-workspace-timeout"
title: "R05 workspace tests finish"
kind: slice
status: abandoned
sprint: "platform"
blocked_by: []
tags: []
created_at: "2026-09-05T20:10:00Z"
updated_at: "2026-09-06T18:00:00Z"
---

# R05 workspace tests finish

## Why

Review of [[slice-398-r05]] left O2 unmet: workspace CHECK budget blew after EXPECT matched. R05 / R05.01 stay `done`. Beat the budget, not a new language atom.

## Done

`cargo test --workspace` exits 0 and prints `test result: ok.` inside the oracle CHECK budget. parser fuzz stays green.

## Blocked by

None.

## Non-goals

- **Re-opening [[slice-398-r05]]**: sealed `failed`
- **Minting a new R05 Loop atom**

## Oracle checklist

- [x] O1: R05 parser fuzz stays locked
  CHECK: cargo test -p draconic-parser --lib fuzz
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes

- [ ] O2: workspace tests finish after the R05 Loop
  CHECK: cargo test --workspace
  EXPECT: test result: ok.
  EVIDENCE: unmet exit=101 match=yes bytes=132848
  ABANDON: leftover after --reverify; first CHECK timed out at 600s (exit=timeout match=yes); 20m reverify failed (exit=101 match=yes bytes=132848); home [[ticket-214-r05-workspace-timeout]]

## Pool

- `[[task-573-r05-workspace-timeout]]`

## See also

ROADMAP.md R05, [[ticket-197-r05-workspace-timeout]], [[slice-398-r05]].
