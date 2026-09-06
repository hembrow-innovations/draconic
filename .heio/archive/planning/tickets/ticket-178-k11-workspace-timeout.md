---
id: "ticket-178-k11-workspace-timeout"
title: "K11 workspace tests did not finish (O2 timeout)"
kind: ticket
status: closed
ticket_type: feature
labels: feature
tags: []
sprint: "platform"
created_at: "2026-09-04T19:35:18Z"
updated_at: "2026-09-06T18:00:00Z"
slice: "slice-348-k11-workspace-timeout"
---

# K11 workspace tests did not finish (O2 timeout)

- **caused-by**: slice-349-k11
- **failed oracle**: O2
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=timeout match=yes bytes=20255 at=2026-09-04T19:34:27.813Z
- **O1**: met (`cargo test -p draconic-pkg k11`)
- **Roadmap ID**: K11
- **Item**: Post-v1 packaging (not v1 bar)
- **Tests**: `crates/draconic-pkg`
- **Targets**: compiler
