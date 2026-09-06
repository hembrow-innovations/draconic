---
id: "ticket-148-h02-workspace-timeout"
title: "H02 workspace tests did not finish (O2 timeout)"
kind: ticket
status: closed
ticket_type: feature
labels: feature
tags: []
sprint: "platform"
created_at: "2026-09-04T16:01:16Z"
updated_at: "2026-09-06T18:00:00Z"
slice: "slice-288-h02-workspace-timeout"
---

# H02 workspace tests did not finish (O2 timeout)

- **caused-by**: slice-289-h02
- **failed oracle**: O2
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=timeout match=yes bytes=86411 at=2026-09-04T16:00:22.333Z
- **O1**: met (`cargo test -p draconic-conformance --test host_stdio`)
- **Roadmap ID**: H02
- **Item**: Stdio: stdout / stderr / stdin
- **Tests**: `tests/conformance/host/stdio`, `crates/draconic-runtime`
- **Targets**: both
