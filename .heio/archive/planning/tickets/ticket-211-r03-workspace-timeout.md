---
id: "ticket-211-r03-workspace-timeout"
title: "R03 workspace tests did not finish (O2 oracle-budget / workspace-timeout)"
kind: ticket
status: closed
ticket_type: bug
labels: bug
tags: []
sprint: "platform"
created_at: "2026-09-05T21:15:00Z"
updated_at: "2026-09-09T09:15:36Z"
---
# R03 workspace tests did not finish (O2 oracle-budget / workspace-timeout)

Reviewer miss on [[slice-391-r03-workspace-timeout]]. This is a budget miss, not a new ROADMAP atom. R03 stays `done` on ROADMAP.md.

- **caused-by**: slice-391-r03-workspace-timeout
- **failed oracle**: O2
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=timeout match=yes bytes=76086
- **O1**: met (`cargo test -p draconic-integration-tests --test supply_chain`)
- **gap**: O2 matched EXPECT then blew the 600s CHECK budget. Not a product fail. Not a new R03 Loop atom.
- **beat**: `cargo test --workspace` exits 0 and prints `test result: ok.` inside the oracle CHECK budget. supply_chain stays green.
