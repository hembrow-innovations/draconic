---
id: "s-l10-workspace-timeout"
title: "L10 workspace tests finish"
kind: slice
status: released
sprint: "platform"
tags: []
created_at: "2026-09-05T20:10:00Z"
updated_at: "2026-09-05T20:29:12Z"
---

# L10 workspace tests finish

## Why

Review of [[s-l10]] left O3 unmet: workspace CHECK budget blew after EXPECT matched. L10 stays `done`. Beat the budget, not a new language atom.

## Done

`cargo test --workspace` exits 0 and prints `test result: ok.` inside the oracle CHECK budget. stdlib_crypto hmac and aead stay green.

## Blocked by

None.

## Non-goals

- **Re-opening [[s-l10]]**: sealed `failed`
- **Minting a new L10 Loop atom**

## Oracle checklist

- [ ] O1: L10 hmac_sha256 fixtures stay locked
  CHECK: cargo test -p draconic-conformance --test stdlib_crypto hmac_sha256
  EXPECT: test result: ok.
  EVIDENCE: pending

- [ ] O2: L10 aead fixtures stay locked
  CHECK: cargo test -p draconic-conformance --test stdlib_crypto aead
  EXPECT: test result: ok.
  EVIDENCE: pending

- [ ] O3: workspace tests finish after the L10 Loop
  CHECK: cargo test --workspace
  EXPECT: test result: ok.
  EVIDENCE: pending

## Pool

- `[[l10-workspace-timeout]]`

## See also

ROADMAP.md L10, [[ticket-200-l10-workspace-timeout]], [[s-l10]].
