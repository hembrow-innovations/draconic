---
id: "slice-372-l09-workspace-timeout"
title: "L09 workspace tests finish"
kind: slice
status: abandoned
sprint: "platform"
blocked_by: []
tags: []
created_at: "2026-09-05T20:10:00Z"
updated_at: "2026-09-06T18:00:00Z"
---

# L09 workspace tests finish

## Why

Review of [[slice-373-l09]] left O2 unmet: `cargo test --workspace` matched EXPECT but blew the CHECK budget. L09 stays `done`. Beat the workspace budget, not a new language atom.

## Done

`cargo test --workspace` exits 0 and prints `test result: ok.` inside the oracle CHECK budget. stdlib_mime stays green.

## Blocked by

None.

## Non-goals

- **Re-opening [[slice-373-l09]]**: sealed `failed`
- **Minting a new L09 Loop atom**

## Oracle checklist

- [x] O1: L09 MIME fixtures stay locked
  CHECK: cargo test -p draconic-conformance --test stdlib_mime
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes

- [ ] O2: workspace tests finish after the L09 Loop
  CHECK: cargo test --workspace
  EXPECT: test result: ok.
  EVIDENCE: unmet exit=timeout match=yes
  ABANDON: leftover after --reverify; CHECK timed out at 600s (exit=timeout match=yes); home [[ticket-206-l09-workspace-timeout]]

## Pool

- `[[task-548-l09-workspace-timeout]]`

## See also

ROADMAP.md L09, [[ticket-192-l09-workspace-timeout]], [[slice-373-l09]].
