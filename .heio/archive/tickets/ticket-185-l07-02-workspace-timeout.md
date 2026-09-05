---
id: "ticket-185-l07-02-workspace-timeout"
title: "L07.02 workspace tests did not finish (O2 timeout)"
kind: ticket
status: closed
labels: feature
tags: []
sprint: platform
created_at: "2026-09-05T00:39:29Z"
updated_at: "2026-09-05T05:08:49Z"
slice: "s-l07-02-workspace-timeout"
caused-by: s-l07-02
failed: true
intent: fix
---

# L07.02 workspace tests did not finish (O2 timeout)

- **caused-by**: s-l07-02
- **failed oracle**: O2
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=timeout match=yes bytes=61436 at=2026-09-05T00:36:29.210Z
- **O1**: met (`cargo test -p draconic-conformance --test stdlib_flags`)
- **Roadmap ID**: L07.02
- **Item**: Typed options (bool/string/number); help text as designed
- **Tests**: `tests/conformance` fixtures `stdlib/flags`
- **Targets**: both
