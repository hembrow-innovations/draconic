---
id: "ticket-162-h16-workspace-timeout"
title: "H16 workspace tests did not finish (O2 timeout)"
kind: ticket
status: closed
ticket_type: feature
labels: feature
tags: []
sprint: "platform"
created_at: "2026-09-04T17:48:04Z"
updated_at: "2026-09-06T18:00:00Z"
slice: "slice-316-h16-workspace-timeout"
---

# H16 workspace tests did not finish (O2 timeout)

- **caused-by**: slice-317-h16
- **failed oracle**: O2
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=timeout match=yes bytes=78592 at=2026-09-04T17:47:15.789Z
- **O1**: met (`cargo test -p draconic-conformance --test host_os`)
- **Roadmap ID**: H16
- **Item**: OS misc
- **Tests**: `tests/conformance/host/os`
- **Targets**: both
