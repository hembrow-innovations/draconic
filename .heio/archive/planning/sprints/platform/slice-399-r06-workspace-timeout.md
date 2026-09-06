---
id: "slice-399-r06-workspace-timeout"
title: "R06 workspace tests finish"
kind: slice
status: abandoned
sprint: "platform"
blocked_by: []
tags: []
created_at: "2026-09-05T20:10:00Z"
updated_at: "2026-09-06T18:00:00Z"
---

# R06 workspace tests finish

## Why

Review of [[slice-400-r06]] left O3 unmet: workspace CHECK budget blew after EXPECT matched. R06 stays `done`. Beat the budget, not a new language atom.

## Done

`cargo test --workspace` exits 0 and prints `test result: ok.` inside the oracle CHECK budget. backtrace lib and panic_backtrace stay green.

## Blocked by

None.

## Non-goals

- **Re-opening [[slice-400-r06]]**: sealed `failed`
- **Minting a new R06 Loop atom**

## Oracle checklist

- [x] O1: R06 backtrace lib tests stay locked
  CHECK: cargo test -p draconic-runtime --lib backtrace
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=fab86a60f32e97af bytes=405

- [x] O2: R06 panic backtrace integration stays locked
  CHECK: cargo test -p draconic-integration-tests --test panic_backtrace
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=2ef5bf887471be30 bytes=2908

- [ ] O3: workspace tests finish after the R06 Loop
  CHECK: cargo test --workspace
  EXPECT: test result: ok.
  EVIDENCE: unmet exit=timeout match=yes bytes=82684
  ABANDON: leftover after --reverify; CHECK timed out at 600s (exit=timeout match=yes bytes=82684); home [[ticket-215-r06-workspace-timeout]]

## Pool

- `[[task-575-r06-workspace-timeout]]`

## See also

ROADMAP.md R06, [[ticket-199-r06-workspace-timeout]], [[slice-400-r06]].
