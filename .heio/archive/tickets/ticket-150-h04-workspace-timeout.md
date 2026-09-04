---
id: "ticket-150-h04-workspace-timeout"
title: "H04 workspace tests did not finish (O2 timeout)"
kind: ticket
status: closed
labels: feature
tags: []
sprint: platform
created_at: "2026-09-04T16:18:40Z"
updated_at: "2026-09-04T20:50:41.788Z"
slice: "s-h04-workspace-timeout"
caused-by: s-h04
failed: true
intent: fix
claimed-by: d0577339-9570-4ae2-8df9-356a99929efa
---

# H04 workspace tests did not finish (O2 timeout)

- **caused-by**: s-h04
- **failed oracle**: O2
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=timeout match=yes bytes=86018 at=2026-09-04T16:17:54.449Z
- **O1**: met (`cargo test -p draconic-conformance --test host_fs`)
- **Roadmap ID**: H04
- **Item**: Filesystem: read / write / dirs
- **Tests**: `tests/conformance/host/fs`, `crates/draconic-backend-llvm`
- **Targets**: both
