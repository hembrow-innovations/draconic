---
id: "ticket-148-h02-workspace-timeout"
title: "H02 workspace tests did not finish (O2 timeout)"
kind: ticket
status: closed
labels: feature
tags: []
sprint: platform
created_at: "2026-09-04T16:01:16Z"
updated_at: "2026-09-04T20:50:41.788Z"
slice: "s-h02-workspace-timeout"
caused-by: s-h02
failed: true
intent: fix
claimed-by: 272f26b2-2dfb-4b22-a73e-c4c613ad602b
---

# H02 workspace tests did not finish (O2 timeout)

- **caused-by**: s-h02
- **failed oracle**: O2
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=timeout match=yes bytes=86411 at=2026-09-04T16:00:22.333Z
- **O1**: met (`cargo test -p draconic-conformance --test host_stdio`)
- **Roadmap ID**: H02
- **Item**: Stdio: stdout / stderr / stdin
- **Tests**: `tests/conformance/host/stdio`, `crates/draconic-runtime`
- **Targets**: both
