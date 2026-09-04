---
id: "ticket-161-h15-workspace-timeout"
title: "H15 workspace tests did not finish (O2 timeout)"
kind: ticket
status: closed
labels: feature
tags: []
sprint: platform
created_at: "2026-09-04T17:40:44Z"
updated_at: "2026-09-04T20:50:41.788Z"
slice: "s-h15-workspace-timeout"
caused-by: s-h15
failed: true
intent: fix
claimed-by: 2a0f7d87-1a97-4162-8b23-b35ba3d2e407
---

# H15 workspace tests did not finish (O2 timeout)

- **caused-by**: s-h15
- **failed oracle**: O2
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=timeout match=yes bytes=86426 at=2026-09-04T17:39:47.055Z
- **O1**: met (`cargo test -p draconic-conformance --test host_process`)
- **Roadmap ID**: H15
- **Item**: Subprocess
- **Tests**: `tests/conformance/host/process/subprocess`
- **Targets**: both
