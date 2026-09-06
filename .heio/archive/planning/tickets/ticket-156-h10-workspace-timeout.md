---
id: "ticket-156-h10-workspace-timeout"
title: "H10 workspace tests did not finish (O2 timeout)"
kind: ticket
status: closed
ticket_type: feature
labels: feature
tags: []
sprint: "platform"
created_at: "2026-09-04T17:12:40Z"
updated_at: "2026-09-06T18:00:00Z"
slice: "slice-304-h10-workspace-timeout"
---

# H10 workspace tests did not finish (O2 timeout)

- **caused-by**: slice-305-h10
- **failed oracle**: O2
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=timeout match=yes bytes=66708 at=2026-09-04T17:11:37.221Z
- **O1**: met (`cargo test -p draconic-conformance --test host_http`)
- **Roadmap ID**: H10
- **Item**: HTTP/1.1 thin helpers (plaintext) on sockets
- **Tests**: `tests/conformance/host/http`
- **Targets**: native
