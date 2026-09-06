---
id: "ticket-150-h04-workspace-timeout"
title: "H04 workspace tests did not finish (O2 timeout)"
kind: ticket
status: closed
ticket_type: feature
labels: feature
tags: []
sprint: "platform"
created_at: "2026-09-04T16:18:40Z"
updated_at: "2026-09-06T18:00:00Z"
slice: "slice-292-h04-workspace-timeout"
---

# H04 workspace tests did not finish (O2 timeout)

- **caused-by**: slice-293-h04
- **failed oracle**: O2
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=timeout match=yes bytes=86018 at=2026-09-04T16:17:54.449Z
- **O1**: met (`cargo test -p draconic-conformance --test host_fs`)
- **Roadmap ID**: H04
- **Item**: Filesystem: read / write / dirs
- **Tests**: `tests/conformance/host/fs`, `crates/draconic-backend-llvm`
- **Targets**: both
