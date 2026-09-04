---
id: "ticket-128-d03-01-workspace-timeout"
title: "D03.01 workspace tests did not finish (O2 timeout)"
kind: ticket
status: closed
labels: feature
tags: []
sprint: platform
created_at: "2026-09-04T14:11:12Z"
updated_at: "2026-09-04T20:50:41.788Z"
slice: "s-d03-01-workspace-timeout"
caused-by: s-d03-01
failed: true
intent: fix
claimed-by: 5cee1d40-3c60-401f-a7e9-e82eae7fc3e9
---

# D03.01 workspace tests did not finish (O2 timeout)

- **caused-by**: s-d03-01
- **failed oracle**: O2
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=timeout match=yes bytes=78599 at=2026-09-04T14:10:32.450Z
- **O1**: met (`cargo test -p draconic-integration-tests --test reproducibility_expectations`)
- **Roadmap ID**: D03.01
- **Item**: Document reproducibility expectations (timestamps, paths)
- **Tests**: `tests/integration`, docs
- **Targets**: compiler
