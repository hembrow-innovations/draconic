---
id: "ticket-136-d05-workspace-timeout"
title: "D05 workspace tests did not finish (O2 timeout)"
kind: ticket
status: closed
labels: feature
tags: []
sprint: platform
created_at: "2026-09-04T14:57:06Z"
updated_at: "2026-09-04T20:50:41.788Z"
slice: "s-d05-workspace-timeout"
caused-by: s-d05
failed: true
intent: fix
claimed-by: edcc938e-695f-41de-808b-571f91e86ea6
---

# D05 workspace tests did not finish (O2 timeout)

- **caused-by**: s-d05
- **failed oracle**: O2
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=timeout match=yes bytes=79148 at=2026-09-04T14:56:39.079Z
- **O1**: met (`cargo test -p draconic-integration-tests --test binary_size`)
- **Roadmap ID**: D05
- **Item**: Binary size opts: strip / LTO flags documented and testable
- **Tests**: `tests/integration`, `crates/draconic-cli`
- **Targets**: native
