---
id: "ticket-158-h12-workspace-timeout"
title: "H12 workspace tests did not finish (O2 timeout)"
kind: ticket
status: closed
ticket_type: feature
labels: feature
tags: []
sprint: "platform"
created_at: "2026-09-04T17:21:30Z"
updated_at: "2026-09-06T18:00:00Z"
slice: "slice-308-h12-workspace-timeout"
---

# H12 workspace tests did not finish (O2 timeout)

- **caused-by**: slice-309-h12
- **failed oracle**: O2
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=timeout match=yes bytes=86470 at=2026-09-04T17:20:49.880Z
- **O1**: met (`cargo test -p draconic-conformance --test host_ws`)
- **Roadmap ID**: H12
- **Item**: WebSocket
- **Tests**: `tests/conformance/host/net/ws`
- **Targets**: native
