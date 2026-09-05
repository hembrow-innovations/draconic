---
id: "s-p04-workspace-tests"
title: "P04 workspace tests pass"
kind: slice
status: failed
sprint: "platform"
tags: []
created_at: "2026-09-05T20:10:00Z"
updated_at: "2026-09-05T21:15:00Z"
---

# P04 workspace tests pass

## Why

Review of [[s-p04]] left O2 unmet: `cargo test --workspace` exited 101. P04 stays `done`. Beat the workspace oracle, not a new language atom.

## Done

`cargo test --workspace` exits 0 and prints `test result: ok.` flagship_service stays green.

## Blocked by

None.

## Non-goals

- **Re-opening [[s-p04]]**: sealed `failed`
- **Minting a new P04 Loop atom**

## Oracle checklist

- [x] O1: P04 flagship service example stays locked
  CHECK: cargo test -p draconic-integration-tests --test flagship_service
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes

- [ ] O2: workspace tests pass after the P04 Loop
  CHECK: cargo test --workspace
  EXPECT: test result: ok.
  EVIDENCE: unmet exit=101 match=yes bytes=132850
  ABANDON: leftover after --reverify; CHECK failed (exit=101 match=yes bytes=132850); cargo test --workspace exited 101 → [[ticket-209-p04-workspace-tests]]

## Pool

- `[[p04-workspace-tests]]`

## See also

ROADMAP.md P04, [[ticket-194-p04-workspace-tests]], [[s-p04]].
