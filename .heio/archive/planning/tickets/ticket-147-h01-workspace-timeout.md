---
id: "ticket-147-h01-workspace-timeout"
title: "H01 workspace tests did not finish (O2 timeout)"
kind: ticket
status: closed
ticket_type: feature
labels: feature
tags: []
sprint: "platform"
created_at: "2026-09-04T15:51:33Z"
updated_at: "2026-09-06T18:00:00Z"
slice: "slice-286-h01-workspace-timeout"
---

# H01 workspace tests did not finish (O2 timeout)

- **caused-by**: slice-287-h01
- **failed oracle**: O2
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=timeout match=yes bytes=78301 at=2026-09-04T15:50:59.808Z
- **O1**: met (`cargo test -p draconic-conformance --test host_process`)
- **Roadmap ID**: H01
- **Item**: Process: args, env, exit
- **Tests**: `tests/conformance/host/process`, `crates/draconic-runtime`
- **Targets**: both
