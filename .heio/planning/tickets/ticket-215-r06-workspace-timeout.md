---
id: "ticket-215-r06-workspace-timeout"
title: "R06 workspace tests did not finish (O3 oracle-budget / workspace-timeout)"
kind: ticket
status: promoted
ticket_type: bug
labels: bug
tags: []
sprint: "platform"
created_at: "2026-09-05T21:15:00Z"
updated_at: "2026-09-06T18:00:00Z"
---

# R06 workspace tests did not finish (O3 oracle-budget / workspace-timeout)

Reviewer miss on [[slice-399-r06-workspace-timeout]]. This is a budget miss, not a new ROADMAP atom. R06 stays `done` on ROADMAP.md.

- **caused-by**: slice-399-r06-workspace-timeout
- **failed oracle**: O3
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=timeout match=yes bytes=82684
- **O1**: met (backtrace lib)
- **O2**: met (panic_backtrace)
- **gap**: O3 matched EXPECT then blew the 600s CHECK budget. Not a product fail. Not a new R06 Loop atom.
- **beat**: `cargo test --workspace` exits 0 and prints `test result: ok.` inside the oracle CHECK budget. backtrace lib and panic_backtrace stay green.
