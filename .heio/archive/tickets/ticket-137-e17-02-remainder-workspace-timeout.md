---
id: "ticket-137-e17-02-remainder-workspace-timeout"
title: "E17.02 remainder workspace tests did not finish (O2 timeout)"
kind: ticket
status: closed
labels: feature
tags: []
sprint: platform
slice: "s-e17-02-remainder-workspace-timeout"
created_at: "2026-09-04T15:04:08Z"
updated_at: "2026-09-04T20:50:41.788Z"
caused-by: s-e17-02-remainder
failed: true
intent: fix
claimed-by: b4a9aabc-26b4-40dd-97dd-813c5dbc654f
---

# E17.02 remainder workspace tests did not finish (O2 timeout)

- **caused-by**: s-e17-02-remainder
- **failed oracle**: O2
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=timeout match=yes bytes=85144 at=2026-09-04T15:03:44.627Z
- **O1**: met (legacy harness `cargo test -p draconic-conformance --test legacy`)
- **Roadmap ID**: E17.02
- **Item**: Other non-strict legacy beyond `with` (children below; untracked remainder stays here)
- **Tests**: `tests/conformance/es/legacy`
- **Targets**: js
