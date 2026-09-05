---
id: "ticket-197-r05-workspace-timeout"
title: "R05 workspace tests did not finish (O2 workspace-timeout)"
kind: ticket
status: ready-for-agent
labels: bug
tags: []
sprint: platform
created_at: "2026-09-05T16:54:28Z"
updated_at: "2026-09-05T16:54:28Z"
caused-by: s-r05
failed: true
intent: fix
---

# R05 workspace tests did not finish (O2 workspace-timeout)

Reviewer miss on [[s-r05]]. This is a budget miss, not a new ROADMAP atom. R05 / R05.01 stay `done` on ROADMAP.md. Next Builder must beat the oracle budget on `cargo test --workspace`, not re-open the language row.

- **caused-by**: s-r05
- **failed oracle**: O2
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=timeout match=yes bytes=120263 at=2026-09-05T16:50:50.583Z
- **O1**: met (`cargo test -p draconic-parser --lib fuzz`)
- **gap**: O2 matched EXPECT (`test result: ok.`) but the oracle CHECK budget blew (`exit=timeout`). Not a product fail. Not a new R05 Loop atom.
- **beat**: `cargo test --workspace` exits 0 and prints `test result: ok.` inside the oracle CHECK budget. O1 stays green.
