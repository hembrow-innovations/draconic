---
id: "ticket-200-l10-workspace-timeout"
title: "L10 workspace tests did not finish (O3 workspace-timeout)"
kind: ticket
status: ready-for-agent
labels: bug
tags: []
sprint: platform
created_at: "2026-09-05T18:21:49Z"
updated_at: "2026-09-05T18:21:49Z"
caused-by: s-l10
failed: true
intent: fix
---

# L10 workspace tests did not finish (O3 workspace-timeout)

Reviewer miss on [[s-l10]]. This is a budget miss, not a new ROADMAP atom. L10 stays `done` on ROADMAP.md. Next Builder must beat the oracle budget on `cargo test --workspace`, not re-open the language row.

- **caused-by**: s-l10
- **failed oracle**: O3
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=timeout match=yes bytes=176939 at=2026-09-05T18:21:07.860Z
- **O1**: met (`cargo test -p draconic-conformance --test stdlib_crypto` hmac_sha256)
- **O2**: met (`cargo test -p draconic-conformance --test stdlib_crypto` aead)
- **gap**: O3 matched EXPECT (`test result: ok.`) but the oracle CHECK budget blew (`exit=timeout`). Not a product fail. Not a new L10 Loop atom.
- **beat**: `cargo test --workspace` exits 0 and prints `test result: ok.` inside the oracle CHECK budget. O1/O2 stay green.
