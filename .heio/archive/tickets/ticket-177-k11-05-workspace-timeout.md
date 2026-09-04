---
id: "ticket-177-k11-05-workspace-timeout"
title: "K11.05 workspace tests did not finish (O2 timeout)"
kind: ticket
status: closed
labels: feature
tags: []
sprint: platform
created_at: "2026-09-04T19:30:51Z"
updated_at: "2026-09-04T20:50:41.788Z"
slice: "s-k11-05-workspace-timeout"
caused-by: s-k11-05
failed: true
intent: fix
claimed-by: d121de3c-5eea-4353-9feb-5275bfb9c66d
---

# K11.05 workspace tests did not finish (O2 timeout)

- **caused-by**: s-k11-05
- **failed oracle**: O2
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=timeout match=yes bytes=79948 at=2026-09-04T19:29:45.695Z
- **O1**: met (`cargo test -p draconic-pkg yank`)
- **Roadmap ID**: K11.05
- **Item**: Yank/retract when advisory source configured
- **Tests**: `crates/draconic-pkg`
- **Targets**: compiler
