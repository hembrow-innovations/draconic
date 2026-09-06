---
id: "ticket-127-d02-workspace-timeout"
title: "D02 workspace tests did not finish (O3 timeout)"
kind: ticket
status: closed
ticket_type: feature
labels: feature
tags: []
sprint: "platform"
created_at: "2026-09-04T14:04:16Z"
updated_at: "2026-09-06T18:00:00Z"
slice: "slice-241-d02-workspace-timeout"
---

# D02 workspace tests did not finish (O3 timeout)

- **caused-by**: slice-242-d02
- **failed oracle**: O3
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=timeout match=yes bytes=79773 at=2026-09-04T14:03:25.091Z
- **O1**: met (`cargo test -p draconic-cli --test toolchain_pin`)
- **O2**: met (`cargo test -p draconic-integration-tests --test toolchain_pin`)
- **Roadmap ID**: D02
- **Item**: Toolchain version pin in `draconic.toml`; CLI enforces or warns
- **Tests**: `crates/draconic-cli`, `tests/integration`
- **Targets**: compiler
