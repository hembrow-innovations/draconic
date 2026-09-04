---
id: "ticket-174-k11-02-workspace-timeout"
title: "K11.02 workspace tests did not finish (O2 timeout)"
kind: ticket
status: closed
labels: feature
tags: []
sprint: platform
created_at: "2026-09-04T19:07:19Z"
updated_at: "2026-09-04T20:50:41.788Z"
slice: "s-k11-02-workspace-timeout"
caused-by: s-k11-02
failed: true
intent: fix
claimed-by: 1c0a415c-cc78-4b1b-82fc-c52ff039718b
---

# K11.02 workspace tests did not finish (O2 timeout)

- **caused-by**: s-k11-02
- **failed oracle**: O2
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=timeout match=yes bytes=88333 at=2026-09-04T19:06:42.751Z
- **O1**: met (`cargo test -p draconic-pkg replace`)
- **Roadmap ID**: K11.02
- **Item**: `replace` directive: fork/local override
- **Tests**: `crates/draconic-pkg`
- **Targets**: compiler
