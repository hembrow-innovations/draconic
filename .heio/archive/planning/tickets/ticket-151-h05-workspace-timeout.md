---
id: "ticket-151-h05-workspace-timeout"
title: "H05 workspace tests did not finish (O2 timeout)"
kind: ticket
status: closed
ticket_type: feature
labels: feature
tags: []
sprint: "platform"
created_at: "2026-09-04T16:27:20Z"
updated_at: "2026-09-06T18:00:00Z"
slice: "slice-294-h05-workspace-timeout"
---

# H05 workspace tests did not finish (O2 timeout)

- **caused-by**: slice-295-h05
- **failed oracle**: O2
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=timeout match=yes bytes=77771 at=2026-09-04T16:26:01.060Z
- **O1**: met (`cargo test -p draconic-conformance --test host_time`)
- **Roadmap ID**: H05
- **Item**: Time, clock, timers (job-queue integrated)
- **Tests**: `tests/conformance/host/time`, `crates/draconic-backend-llvm`, `crates/draconic-runtime`
- **Targets**: both
