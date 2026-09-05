---
id: "ticket-192-l09-workspace-timeout"
title: "L09 workspace tests did not finish (O2 workspace-timeout)"
kind: ticket
status: ready-for-agent
labels: bug
tags: []
sprint: platform
created_at: "2026-09-05T13:45:58Z"
updated_at: "2026-09-05T13:45:58Z"
caused-by: s-l09
failed: true
intent: fix
---

# L09 workspace tests did not finish (O2 workspace-timeout)

Reviewer miss on [[s-l09]]. This is a budget miss, not a new ROADMAP atom. L09 stays `done` on ROADMAP.md. Next Builder must beat the oracle budget on `cargo test --workspace`, not re-open the language row.

- **caused-by**: s-l09
- **failed oracle**: O2
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=timeout match=yes bytes=157001 at=2026-09-05T13:45:03.076Z
- **O1**: met (`cargo test -p draconic-conformance --test stdlib_mime`)
- **gap**: O2 matched EXPECT (`test result: ok.`) but the oracle CHECK budget blew (`exit=timeout`). Not a product fail. Not a new L09 Loop atom.
- **beat**: `cargo test --workspace` exits 0 and prints `test result: ok.` inside the oracle CHECK budget. O1 stays green.
