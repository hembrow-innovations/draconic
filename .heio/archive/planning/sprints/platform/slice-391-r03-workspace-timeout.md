---
id: "slice-391-r03-workspace-timeout"
title: "R03 workspace tests finish"
kind: slice
status: abandoned
sprint: "platform"
blocked_by: []
tags: []
created_at: "2026-09-05T20:10:00Z"
updated_at: "2026-09-06T18:00:00Z"
---

# R03 workspace tests finish

## Why

Review of [[slice-392-r03]] left O2 unmet: workspace CHECK budget blew after EXPECT matched. Do not mint a new R03 Loop atom. Beat the budget.

## Done

`cargo test --workspace` exits 0 and prints `test result: ok.` inside the oracle CHECK budget. supply_chain stays green.

## Blocked by

None.

## Non-goals

- **Re-opening [[slice-392-r03]]**: sealed
- **Minting a new R03 Loop atom**

## Oracle checklist

- [x] O1: R03 supply-chain integration stays locked
  CHECK: cargo test -p draconic-integration-tests --test supply_chain
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=d37eb43d6dd53913 bytes=3010

- [ ] O2: workspace tests finish after the R03 Loop
  CHECK: cargo test --workspace
  EXPECT: test result: ok.
  EVIDENCE: unmet exit=timeout match=yes bytes=76086
  ABANDON: leftover after --reverify; CHECK timed out at 600s (exit=timeout match=yes bytes=76086); home [[ticket-211-r03-workspace-timeout]]

## Pool

- `[[task-567-r03-workspace-timeout]]`

## See also

ROADMAP.md R03, [[ticket-201-r03-workspace-timeout]], [[slice-392-r03]].
