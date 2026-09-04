---
id: "ticket-130-d03-workspace-timeout"
title: "D03 workspace tests did not finish (O2 timeout)"
kind: ticket
status: closed
labels: feature
tags: []
sprint: platform
created_at: "2026-09-04T14:21:45Z"
updated_at: "2026-09-04T20:50:41.788Z"
slice: "s-d03-workspace-timeout"
caused-by: s-d03
failed: true
intent: fix
claimed-by: 0d3874f8-d063-4b85-85f9-8128f4b0dd8b
---

# D03 workspace tests did not finish (O2 timeout)

- **caused-by**: s-d03
- **failed oracle**: O2
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=timeout match=yes bytes=79070 at=2026-09-04T14:21:04.680Z
- **O1**: met (`cargo test -p draconic-integration-tests --test reproducible_builds`)
- **Roadmap ID**: D03
- **Item**: Reproducible builds: same source + pin → documented-equivalent artifacts
- **Tests**: `tests/integration` (`reproducible_builds`)
- **Targets**: compiler
