---
id: "ticket-127-d02-workspace-timeout"
title: "D02 workspace tests did not finish (O3 timeout)"
kind: ticket
status: closed
labels: feature
tags: []
sprint: platform
created_at: "2026-09-04T14:04:16Z"
updated_at: "2026-09-04T20:50:41.788Z"
slice: "s-d02-workspace-timeout"
caused-by: s-d02
failed: true
intent: fix
claimed-by: 4323ba5d-ec65-4cfa-8d6f-dccbee1e30f2
---

# D02 workspace tests did not finish (O3 timeout)

- **caused-by**: s-d02
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
