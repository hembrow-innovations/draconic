---
id: "s-r06-workspace-timeout"
title: "R06 workspace tests finish"
kind: slice
status: released
sprint: "platform"
tags: []
created_at: "2026-09-05T20:10:00Z"
updated_at: "2026-09-05T20:29:12Z"
---

# R06 workspace tests finish

## Why

Review of [[s-r06]] left O3 unmet: workspace CHECK budget blew after EXPECT matched. R06 stays `done`. Beat the budget, not a new language atom.

## Done

`cargo test --workspace` exits 0 and prints `test result: ok.` inside the oracle CHECK budget. backtrace lib and panic_backtrace stay green.

## Blocked by

None.

## Non-goals

- **Re-opening [[s-r06]]**: sealed `failed`
- **Minting a new R06 Loop atom**

## Oracle checklist

- [ ] O1: R06 backtrace lib tests stay locked
  CHECK: cargo test -p draconic-runtime --lib backtrace
  EXPECT: test result: ok.
  EVIDENCE: pending

- [ ] O2: R06 panic backtrace integration stays locked
  CHECK: cargo test -p draconic-integration-tests --test panic_backtrace
  EXPECT: test result: ok.
  EVIDENCE: pending

- [ ] O3: workspace tests finish after the R06 Loop
  CHECK: cargo test --workspace
  EXPECT: test result: ok.
  EVIDENCE: pending

## Pool

- `[[r06-workspace-timeout]]`

## See also

ROADMAP.md R06, [[ticket-199-r06-workspace-timeout]], [[s-r06]].
