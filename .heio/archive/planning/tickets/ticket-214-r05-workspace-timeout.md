---
id: "ticket-214-r05-workspace-timeout"
title: "R05 workspace tests timeout then exit 101 (O2 oracle-budget /
  workspace-timeout)"
kind: ticket
status: closed
ticket_type: bug
labels: bug
tags: []
sprint: "platform"
created_at: "2026-09-05T21:15:00Z"
updated_at: "2026-09-09T09:15:36Z"
---
# R05 workspace tests timeout then exit 101 (O2 oracle-budget / workspace-timeout)

Reviewer miss on [[slice-397-r05-workspace-timeout]]. First CHECK was a budget miss; a 20m reverify then exited 101. R05 / R05.01 stay `done` on ROADMAP.md. Not a new language atom.

- **caused-by**: slice-397-r05-workspace-timeout
- **failed oracle**: O2
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=101 match=yes bytes=132848 (after 600s timeout match=yes)
- **O1**: met (`cargo test -p draconic-parser --lib fuzz`)
- **gap**: O2 first blew the 600s CHECK budget with match=yes; a longer reverify then exited 101 with EXPECT substring present. Not a new R05 Loop atom.
- **beat**: `cargo test --workspace` exits 0 and prints `test result: ok.` inside the oracle CHECK budget. parser fuzz stays green.
