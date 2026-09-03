---
id: "ticket-28-e17-02-workspace-timeout"
title: "E17.02 remainder workspace tests did not finish (O2 timeout)"
kind: ticket
status: closed
labels: feature
tags: []
sprint: platform
slice: "s-e17-02-workspace-timeout"
created_at: "2026-09-02T09:14:13Z"
updated_at: "2026-09-03T05:16:34Z"
caused-by: s-e17-02
failed: true
intent: fix
---

# E17.02 remainder workspace tests did not finish (O2 timeout)

- **caused-by**: s-e17-02
- **failed oracle**: O2
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=timeout match=yes bytes=69001 at=2026-09-02T09:13:24.914Z
- **O1**: met (legacy harness `cargo test -p draconic-conformance --test legacy`)
- **Roadmap ID**: E17.02
- **Item**: Other non-strict legacy beyond `with` (children below; untracked remainder stays here)
- **Tests**: `tests/conformance/es/legacy`
- **Targets**: js
