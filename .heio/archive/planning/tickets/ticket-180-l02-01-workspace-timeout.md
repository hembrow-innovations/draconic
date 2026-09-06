---
id: "ticket-180-l02-01-workspace-timeout"
title: "L02.01 workspace tests did not finish (O2 timeout)"
kind: ticket
status: closed
ticket_type: feature
labels: feature
tags: []
sprint: "platform"
created_at: "2026-09-04T19:48:06Z"
updated_at: "2026-09-06T18:00:00Z"
slice: "slice-352-l02-01-workspace-timeout"
---

# L02.01 workspace tests did not finish (O2 timeout)

- **caused-by**: slice-353-l02-01
- **failed oracle**: O2
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=timeout match=yes bytes=64131 at=2026-09-04T19:47:32.712Z
- **O1**: met (`cargo test -p draconic-conformance --test stdlib_collections`)
- **Roadmap ID**: L02.01
- **Item**: `groupBy` / `chunk` (or designed names) on arrays
- **Tests**: `tests/conformance` fixtures `stdlib/collections`
- **Targets**: both
