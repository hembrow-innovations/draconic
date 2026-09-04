---
id: "ticket-134-d05-01-workspace-timeout"
title: "D05.01 workspace tests did not finish (O3 timeout)"
kind: ticket
status: closed
labels: feature
tags: []
sprint: platform
created_at: "2026-09-04T14:41:04Z"
updated_at: "2026-09-04T20:50:41.788Z"
slice: "s-d05-01-workspace-timeout"
caused-by: s-d05-01
failed: true
intent: fix
claimed-by: 4f6648c2-3c09-4faa-9074-894913a77311
---

# D05.01 workspace tests did not finish (O3 timeout)

- **caused-by**: s-d05-01
- **failed oracle**: O3
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=timeout match=yes bytes=78488 at=2026-09-04T14:40:16.603Z
- **O1**: met (`cargo test -p draconic-cli --test strip_symbols`)
- **O2**: met (`cargo test -p draconic-integration-tests --test binary_size_strip`)
- **Roadmap ID**: D05.01
- **Item**: CLI/build flags: strip symbols
- **Tests**: `crates/draconic-cli`, `tests/integration`
- **Targets**: native
