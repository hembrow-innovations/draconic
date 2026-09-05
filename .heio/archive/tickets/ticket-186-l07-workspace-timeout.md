---
id: "ticket-186-l07-workspace-timeout"
title: "L07 workspace tests did not finish (O2 timeout)"
kind: ticket
status: closed
labels: feature
tags: []
sprint: platform
created_at: "2026-09-05T00:52:07Z"
updated_at: "2026-09-05T20:00:45Z"
slice: "s-l07-workspace-timeout"
caused-by: s-l07
failed: true
intent: fix
---

# L07 workspace tests did not finish (O2 timeout)

- **caused-by**: s-l07
- **failed oracle**: O2
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=timeout match=yes bytes=69362 at=2026-09-05T00:51:18.437Z
- **O1**: met (`cargo test -p draconic-conformance --test stdlib_flags`)
- **Roadmap ID**: L07
- **Item**: Flags/CLI parse: argv → typed options/positionals
- **Tests**: `tests/conformance` fixtures `stdlib/flags`
- **Targets**: both
