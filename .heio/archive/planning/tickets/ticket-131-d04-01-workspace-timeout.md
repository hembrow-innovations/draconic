---
id: "ticket-131-d04-01-workspace-timeout"
title: "D04.01 workspace tests did not finish (O2 timeout)"
kind: ticket
status: closed
ticket_type: feature
labels: feature
tags: []
sprint: "platform"
created_at: "2026-09-04T14:25:38Z"
updated_at: "2026-09-06T18:00:00Z"
slice: "slice-249-d04-01-workspace-timeout"
---

# D04.01 workspace tests did not finish (O2 timeout)

- **caused-by**: slice-250-d04-01
- **failed oracle**: O2
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=timeout match=yes bytes=77420 at=2026-09-04T14:24:34.371Z
- **O1**: met (`cargo test -p draconic-integration-tests --test cross_compile_non_host`)
- **Roadmap ID**: D04.01
- **Item**: Cross-compile: at least one non-host triple smoke
- **Tests**: `tests/integration`, `crates/draconic-backend-llvm`
- **Targets**: native
