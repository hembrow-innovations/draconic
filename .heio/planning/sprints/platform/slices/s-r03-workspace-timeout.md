---
id: "s-r03-workspace-timeout"
title: "R03 workspace tests finish"
kind: slice
status: released
sprint: "platform"
tags: []
created_at: "2026-09-05T20:10:00Z"
updated_at: "2026-09-05T20:29:12Z"
---

# R03 workspace tests finish

## Why

Review of [[s-r03]] left O2 unmet: workspace CHECK budget blew after EXPECT matched. Do not mint a new R03 Loop atom. Beat the budget.

## Done

`cargo test --workspace` exits 0 and prints `test result: ok.` inside the oracle CHECK budget. supply_chain stays green.

## Blocked by

None.

## Non-goals

- **Re-opening [[s-r03]]**: sealed
- **Minting a new R03 Loop atom**

## Oracle checklist

- [ ] O1: R03 supply-chain integration stays locked
  CHECK: cargo test -p draconic-integration-tests --test supply_chain
  EXPECT: test result: ok.
  EVIDENCE: pending

- [ ] O2: workspace tests finish after the R03 Loop
  CHECK: cargo test --workspace
  EXPECT: test result: ok.
  EVIDENCE: pending

## Pool

- `[[r03-workspace-timeout]]`

## See also

ROADMAP.md R03, [[ticket-201-r03-workspace-timeout]], [[s-r03]].
