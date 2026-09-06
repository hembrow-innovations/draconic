---
id: "ticket-175-k11-03-workspace-timeout"
title: "K11.03 workspace tests did not finish (O2 timeout)"
kind: ticket
status: closed
ticket_type: feature
labels: feature
tags: []
sprint: "platform"
created_at: "2026-09-04T19:15:03Z"
updated_at: "2026-09-06T18:00:00Z"
slice: "slice-342-k11-03-workspace-timeout"
---

# K11.03 workspace tests did not finish (O2 timeout)

- **caused-by**: slice-343-k11-03
- **failed oracle**: O2
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=timeout match=yes bytes=77915 at=2026-09-04T19:14:32.875Z
- **O1**: met (`cargo test -p draconic-pkg subdir`)
- **Roadmap ID**: K11.03
- **Item**: Multi-module monorepo (subdir module paths)
- **Tests**: `crates/draconic-pkg`
- **Targets**: compiler
