---
id: "ticket-212-r04-workspace-timeout"
title: "R04 workspace tests did not finish (O3 oracle-budget / workspace-timeout)"
kind: ticket
status: ready-for-agent
labels: bug
tags: []
sprint: platform
created_at: "2026-09-05T21:15:00Z"
updated_at: "2026-09-05T21:15:00Z"
caused-by: s-r04-workspace-timeout
failed: true
intent: fix
---

# R04 workspace tests did not finish (O3 oracle-budget / workspace-timeout)

Reviewer miss on [[s-r04-workspace-timeout]]. This is a budget miss, not a new ROADMAP atom. R04 / R04.01 / R04.02 stay `done` on ROADMAP.md.

- **caused-by**: s-r04-workspace-timeout
- **failed oracle**: O3
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=timeout match=yes bytes=80084
- **O1**: met (abort_policy)
- **O2**: met (panic_policy)
- **gap**: O3 matched EXPECT then blew the 600s CHECK budget. Not a product fail. Not a new R04 Loop atom.
- **beat**: `cargo test --workspace` exits 0 and prints `test result: ok.` inside the oracle CHECK budget. abort_policy and panic_policy stay green.
