---
id: "ticket-177-k11-05-workspace-timeout"
title: "K11.05 workspace tests did not finish (O2 timeout)"
kind: ticket
status: closed
ticket_type: feature
labels: feature
tags: []
sprint: "platform"
created_at: "2026-09-04T19:30:51Z"
updated_at: "2026-09-06T18:00:00Z"
slice: "slice-346-k11-05-workspace-timeout"
---

# K11.05 workspace tests did not finish (O2 timeout)

- **caused-by**: slice-347-k11-05
- **failed oracle**: O2
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=timeout match=yes bytes=79948 at=2026-09-04T19:29:45.695Z
- **O1**: met (`cargo test -p draconic-pkg yank`)
- **Roadmap ID**: K11.05
- **Item**: Yank/retract when advisory source configured
- **Tests**: `crates/draconic-pkg`
- **Targets**: compiler
