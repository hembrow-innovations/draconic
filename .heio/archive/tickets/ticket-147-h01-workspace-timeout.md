---
id: "ticket-147-h01-workspace-timeout"
title: "H01 workspace tests did not finish (O2 timeout)"
kind: ticket
status: closed
labels: feature
tags: []
sprint: platform
created_at: "2026-09-04T15:51:33Z"
updated_at: "2026-09-04T20:50:41.788Z"
slice: "s-h01-workspace-timeout"
caused-by: s-h01
failed: true
intent: fix
claimed-by: d804f8a4-1d8e-477d-ba41-a0556d20db9c
---

# H01 workspace tests did not finish (O2 timeout)

- **caused-by**: s-h01
- **failed oracle**: O2
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=timeout match=yes bytes=78301 at=2026-09-04T15:50:59.808Z
- **O1**: met (`cargo test -p draconic-conformance --test host_process`)
- **Roadmap ID**: H01
- **Item**: Process: args, env, exit
- **Tests**: `tests/conformance/host/process`, `crates/draconic-runtime`
- **Targets**: both
