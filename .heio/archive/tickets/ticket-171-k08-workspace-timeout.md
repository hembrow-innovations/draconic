---
id: "ticket-171-k08-workspace-timeout"
title: "K08 workspace tests did not finish (O2 timeout)"
kind: ticket
status: closed
labels: feature
tags: []
sprint: platform
created_at: "2026-09-04T18:55:49Z"
updated_at: "2026-09-04T20:50:41.788Z"
slice: "s-k08-workspace-timeout"
caused-by: s-k08
failed: true
intent: fix
claimed-by: 230f628b-85ff-4eed-a376-e5fb02ee047f
---

# K08 workspace tests did not finish (O2 timeout)

- **caused-by**: s-k08
- **failed oracle**: O2
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=timeout match=yes bytes=76836 at=2026-09-04T18:54:40.556Z
- **O1**: met (`cargo test -p draconic-pkg hash`)
- **Roadmap ID**: K08
- **Item**: Integrity: verify lock hashes; refuse tampered cache
- **Tests**: `crates/draconic-pkg`
- **Targets**: compiler
