---
id: "ticket-158-h12-workspace-timeout"
title: "H12 workspace tests did not finish (O2 timeout)"
kind: ticket
status: closed
labels: feature
tags: []
sprint: platform
created_at: "2026-09-04T17:21:30Z"
updated_at: "2026-09-04T20:50:41.788Z"
slice: "s-h12-workspace-timeout"
caused-by: s-h12
failed: true
intent: fix
claimed-by: c41af2d6-6fe6-4742-a817-194a3131a25a
---

# H12 workspace tests did not finish (O2 timeout)

- **caused-by**: s-h12
- **failed oracle**: O2
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=timeout match=yes bytes=86470 at=2026-09-04T17:20:49.880Z
- **O1**: met (`cargo test -p draconic-conformance --test host_ws`)
- **Roadmap ID**: H12
- **Item**: WebSocket
- **Tests**: `tests/conformance/host/net/ws`
- **Targets**: native
