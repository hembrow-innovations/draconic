---
id: "ticket-155-h09-workspace-timeout"
title: "H09 workspace tests did not finish (O2 timeout)"
kind: ticket
status: closed
ticket_type: feature
labels: feature
tags: []
sprint: "platform"
created_at: "2026-09-04T17:03:37Z"
updated_at: "2026-09-06T18:00:00Z"
slice: "slice-302-h09-workspace-timeout"
---

# H09 workspace tests did not finish (O2 timeout)

- **caused-by**: slice-303-h09
- **failed oracle**: O2
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=timeout match=yes bytes=85355 at=2026-09-04T17:02:59.794Z
- **O1**: met (`cargo test -p draconic-conformance --test host_dns`)
- **Roadmap ID**: H09
- **Item**: DNS
- **Tests**: `tests/conformance/host/net/dns`
- **Targets**: native
