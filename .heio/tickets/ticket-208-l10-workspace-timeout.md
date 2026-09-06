---
id: "ticket-208-l10-workspace-timeout"
title: "L10 workspace tests did not finish (O3 oracle-budget / workspace-timeout)"
kind: ticket
status: promoted
labels: bug
tags: []
sprint: platform
created_at: "2026-09-05T21:15:00Z"
updated_at: "2026-09-06T04:39:34Z"
caused-by: s-l10-workspace-timeout
failed: true
intent: fix
---

# L10 workspace tests did not finish (O3 oracle-budget / workspace-timeout)

Reviewer miss on [[s-l10-workspace-timeout]]. This is a budget miss, not a new ROADMAP atom. L10 stays `done` on ROADMAP.md.

- **caused-by**: s-l10-workspace-timeout
- **failed oracle**: O3
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=timeout match=yes bytes=76525
- **O1**: met (hmac_sha256)
- **O2**: met (aead)
- **gap**: O3 matched EXPECT then blew the 600s CHECK budget. Not a product fail. Not a new L10 Loop atom.
- **beat**: `cargo test --workspace` exits 0 and prints `test result: ok.` inside the oracle CHECK budget. hmac and aead stay green.
