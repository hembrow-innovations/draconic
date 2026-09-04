---
id: "ticket-156-h10-workspace-timeout"
title: "H10 workspace tests did not finish (O2 timeout)"
kind: ticket
status: closed
labels: feature
tags: []
sprint: platform
created_at: "2026-09-04T17:12:40Z"
updated_at: "2026-09-04T20:50:41.788Z"
slice: "s-h10-workspace-timeout"
caused-by: s-h10
failed: true
intent: fix
claimed-by: 1a7f4504-77b5-45b6-a7d2-318890d8bc5d
---

# H10 workspace tests did not finish (O2 timeout)

- **caused-by**: s-h10
- **failed oracle**: O2
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=timeout match=yes bytes=66708 at=2026-09-04T17:11:37.221Z
- **O1**: met (`cargo test -p draconic-conformance --test host_http`)
- **Roadmap ID**: H10
- **Item**: HTTP/1.1 thin helpers (plaintext) on sockets
- **Tests**: `tests/conformance/host/http`
- **Targets**: native
