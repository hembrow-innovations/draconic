---
id: "ticket-184-l06-workspace-timeout"
title: "L06 workspace tests did not finish (O2 timeout)"
kind: ticket
status: closed
labels: feature
tags: []
sprint: platform
created_at: "2026-09-04T21:00:28Z"
updated_at: "2026-09-05T05:08:49Z"
slice: "s-l06-workspace-timeout"
caused-by: s-l06
failed: true
intent: fix
---

# L06 workspace tests did not finish (O2 timeout)

- **caused-by**: s-l06
- **failed oracle**: O2
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=timeout match=yes bytes=70857 at=2026-09-04T20:59:27.249Z
- **O1**: met (`cargo test -p draconic-conformance --test stdlib_logging`)
- **Roadmap ID**: L06
- **Item**: Logging: leveled logger; stderr/stdout sink
- **Tests**: `tests/conformance` fixtures `stdlib/logging`
- **Targets**: both
