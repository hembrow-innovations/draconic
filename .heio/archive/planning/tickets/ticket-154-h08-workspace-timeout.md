---
id: "ticket-154-h08-workspace-timeout"
title: "H08 workspace tests did not finish (O2 timeout)"
kind: ticket
status: closed
ticket_type: feature
labels: feature
tags: []
sprint: "platform"
created_at: "2026-09-04T16:51:27Z"
updated_at: "2026-09-06T18:00:00Z"
slice: "slice-300-h08-workspace-timeout"
---

# H08 workspace tests did not finish (O2 timeout)

- **caused-by**: slice-301-h08
- **failed oracle**: O2
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=timeout match=yes bytes=79732 at=2026-09-04T16:50:49.086Z
- **O1**: met (`cargo test -p draconic-conformance --test host_udp`)
- **Roadmap ID**: H08
- **Item**: UDP
- **Tests**: `tests/conformance/host/net/udp`
- **Targets**: native
