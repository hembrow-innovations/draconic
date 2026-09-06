---
id: "slice-254-d04-workspace-tests-timeout"
title: "D04 workspace tests finish"
kind: slice
status: abandoned
sprint: "platform"
blocked_by: []
tags: []
created_at: "2026-09-05T20:10:00Z"
updated_at: "2026-09-06T18:00:00Z"
---

# D04 workspace tests finish

## Why

Review of [[slice-255-d04-workspace-tests]] left O1 unmet: `cargo test --workspace` matched EXPECT but blew the CHECK budget (`exit=timeout`). D04 / D04.02 / D01.01 stay `done`. This cut beats the workspace budget, not a new language atom.

## Done

`cargo test --workspace` exits 0 and prints `test result: ok.` inside the oracle CHECK budget. O2/O3/O4 stay green against `.yml.disabled`.

## Blocked by

None.

## Non-goals

- **Re-opening [[slice-255-d04-workspace-tests]]**: sealed `failed`
- **Restoring live `.yml` names**
- **Minting a new D04 Loop atom**

## Oracle checklist

- [ ] O1: workspace tests finish after the D04 Loop
  CHECK: cargo test --workspace
  EXPECT: test result: ok.
  EVIDENCE: unmet exit=timeout match=yes bytes=90371
  ABANDON: leftover after --reverify; CHECK timed out at 600s (exit=timeout match=yes bytes=90371); home [[ticket-204-d04-workspace-tests-timeout]]

- [x] O2: D04 combined matrix sitting stays green
  CHECK: cargo test -p draconic-integration-tests --test cross_compile
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=308a79ecfe0cdbae bytes=2994

- [x] O3: D04.02 matrix docs + CI jobs fixtures stay locked
  CHECK: cargo test -p draconic-integration-tests --test cross_compile_matrix
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=4aa60b6834231799 bytes=2739

- [ ] O4: D01.01 release-artifact workflow reader stays locked
  CHECK: cargo test -p draconic-integration-tests --test release_artifact
  EXPECT: test result: ok.
  EVIDENCE: unmet exit=101 match=no bytes=3329
  ABANDON: leftover after --reverify; CHECK failed (exit=101 match=no bytes=3329); home [[ticket-204-d04-workspace-tests-timeout]]

## Pool

- `[[task-430-d04-workspace-tests-timeout]]`

## See also

ROADMAP.md D04, [[ticket-191-d04-workspace-tests-timeout]], [[slice-255-d04-workspace-tests]].
