---
id: "s-r04-workspace-timeout"
title: "R04 workspace tests finish"
kind: slice
status: released
sprint: "platform"
tags: []
created_at: "2026-09-05T20:10:00Z"
updated_at: "2026-09-05T20:29:12Z"
---

# R04 workspace tests finish

## Why

Review of [[s-r04]] left O3 unmet: workspace CHECK budget blew after EXPECT matched. R04 / R04.01 / R04.02 stay `done`. Beat the budget, not a new language atom.

## Done

`cargo test --workspace` exits 0 and prints `test result: ok.` inside the oracle CHECK budget. abort_policy and panic_policy stay green.

## Blocked by

None.

## Non-goals

- **Re-opening [[s-r04]]**: sealed `failed`
- **Minting a new R04 Loop atom**

## Oracle checklist

- [ ] O1: R04 abort policy lib tests stay locked
  CHECK: cargo test -p draconic-runtime --lib abort_policy
  EXPECT: test result: ok.
  EVIDENCE: pending

- [ ] O2: R04 panic policy conformance stays locked
  CHECK: cargo test -p draconic-conformance --test panic_policy
  EXPECT: test result: ok.
  EVIDENCE: pending

- [ ] O3: workspace tests finish after the R04 Loop
  CHECK: cargo test --workspace
  EXPECT: test result: ok.
  EVIDENCE: pending

## Pool

- `[[r04-workspace-timeout]]`

## See also

ROADMAP.md R04, [[ticket-196-r04-workspace-timeout]], [[s-r04]].
