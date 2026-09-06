---
id: "ticket-161-h15-workspace-timeout"
title: "H15 workspace tests did not finish (O2 timeout)"
kind: ticket
status: closed
ticket_type: feature
labels: feature
tags: []
sprint: "platform"
created_at: "2026-09-04T17:40:44Z"
updated_at: "2026-09-06T18:00:00Z"
slice: "slice-314-h15-workspace-timeout"
---

# H15 workspace tests did not finish (O2 timeout)

- **caused-by**: slice-315-h15
- **failed oracle**: O2
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=timeout match=yes bytes=86426 at=2026-09-04T17:39:47.055Z
- **O1**: met (`cargo test -p draconic-conformance --test host_process`)
- **Roadmap ID**: H15
- **Item**: Subprocess
- **Tests**: `tests/conformance/host/process/subprocess`
- **Targets**: both
