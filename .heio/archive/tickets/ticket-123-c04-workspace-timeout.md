---
id: "ticket-123-c04-workspace-timeout"
title: "C04 workspace tests did not finish (O4 timeout)"
kind: ticket
status: closed
labels: feature
tags: []
sprint: platform
created_at: "2026-09-04T13:37:44Z"
updated_at: "2026-09-04T20:50:41.788Z"
slice: "s-c04-workspace-timeout"
caused-by: s-c04
failed: true
intent: fix
claimed-by: 8f4a864b-32a4-4f08-8edd-b87b4d614083
---

# C04 workspace tests did not finish (O4 timeout)

- **caused-by**: s-c04
- **failed oracle**: O4
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=timeout match=yes bytes=88987 at=2026-09-04T13:36:57.467Z
- **O1**: met (`cargo test -p draconic-cli --test test_cmd`)
- **O2**: met (`cargo test -p draconic-integration-tests --test cli_test_jobs`)
- **O3**: met (`cargo test -p draconic-integration-tests --test cli_test_aggregate_order`)
- **Roadmap ID**: C04
- **Item**: Parallel `draconic test`: multi-fixture workers; deterministic aggregate exit
- **Tests**: `crates/draconic-cli`, `tests/integration`
- **Targets**: compiler
