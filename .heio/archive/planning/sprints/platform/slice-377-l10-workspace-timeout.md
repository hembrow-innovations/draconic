---
id: "slice-377-l10-workspace-timeout"
title: "L10 workspace tests finish"
kind: slice
status: abandoned
sprint: "platform"
blocked_by: []
tags: []
created_at: "2026-09-05T20:10:00Z"
updated_at: "2026-09-06T18:00:00Z"
---

# L10 workspace tests finish

## Why

Review of [[slice-378-l10]] left O3 unmet: workspace CHECK budget blew after EXPECT matched. L10 stays `done`. Beat the budget, not a new language atom.

## Done

`cargo test --workspace` exits 0 and prints `test result: ok.` inside the oracle CHECK budget. stdlib_crypto hmac and aead stay green.

## Blocked by

None.

## Non-goals

- **Re-opening [[slice-378-l10]]**: sealed `failed`
- **Minting a new L10 Loop atom**

## Oracle checklist

- [x] O1: L10 hmac_sha256 fixtures stay locked
  CHECK: cargo test -p draconic-conformance --test stdlib_crypto hmac_sha256
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=6a4171520250f871 bytes=2698

- [x] O2: L10 aead fixtures stay locked
  CHECK: cargo test -p draconic-conformance --test stdlib_crypto aead
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=bdfbc29e925002c2 bytes=2837

- [ ] O3: workspace tests finish after the L10 Loop
  CHECK: cargo test --workspace
  EXPECT: test result: ok.
  EVIDENCE: unmet exit=timeout match=yes bytes=76525
  ABANDON: leftover after --reverify; CHECK timed out at 600s (exit=timeout match=yes bytes=76525); home [[ticket-208-l10-workspace-timeout]]

## Pool

- `[[task-553-l10-workspace-timeout]]`

## See also

ROADMAP.md L10, [[ticket-200-l10-workspace-timeout]], [[slice-378-l10]].
