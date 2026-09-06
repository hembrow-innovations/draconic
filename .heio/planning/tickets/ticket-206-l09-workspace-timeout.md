---
id: "ticket-206-l09-workspace-timeout"
title: "L09 workspace tests did not finish (O2 oracle-budget / workspace-timeout)"
kind: ticket
status: promoted
ticket_type: bug
labels: bug
tags: []
sprint: "platform"
created_at: "2026-09-05T21:15:00Z"
updated_at: "2026-09-06T18:00:00Z"
---

# L09 workspace tests did not finish (O2 oracle-budget / workspace-timeout)

Reviewer miss on [[slice-372-l09-workspace-timeout]]. This is a budget miss, not a new ROADMAP atom. L09 stays `done` on ROADMAP.md.

- **caused-by**: slice-372-l09-workspace-timeout
- **failed oracle**: O2
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=timeout match=yes
- **O1**: met (`cargo test -p draconic-conformance --test stdlib_mime`)
- **gap**: O2 matched EXPECT (`test result: ok.`) but the oracle CHECK budget blew (`exit=timeout`). Not a product fail. Not a new L09 Loop atom.
- **beat**: `cargo test --workspace` exits 0 and prints `test result: ok.` inside the oracle CHECK budget. stdlib_mime stays green.
