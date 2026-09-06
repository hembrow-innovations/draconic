---
id: "ticket-153-h07-workspace-timeout"
title: "H07 workspace tests did not finish (O3 timeout)"
kind: ticket
status: closed
ticket_type: feature
labels: feature
tags: []
sprint: "platform"
created_at: "2026-09-04T16:44:51Z"
updated_at: "2026-09-06T18:00:00Z"
slice: "slice-298-h07-workspace-timeout"
---

# H07 workspace tests did not finish (O3 timeout)

- **caused-by**: slice-299-h07
- **failed oracle**: O3
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=timeout match=yes bytes=85355 at=2026-09-04T16:44:19.884Z
- **O1**: met (`cargo test -p draconic-conformance --test host_tcp_async`)
- **O2**: met (`cargo test -p draconic-runtime --lib`)
- **Roadmap ID**: H07
- **Item**: Async socket I/O + job queue
- **Tests**: `tests/conformance/host/net/async`, `crates/draconic-runtime`
- **Targets**: native
