---
id: "ticket-143-f07-workspace-timeout"
title: "F07 workspace tests did not finish (O3 timeout)"
kind: ticket
status: closed
labels: feature
tags: []
sprint: platform
created_at: "2026-09-04T15:30:59Z"
updated_at: "2026-09-04T20:50:41.788Z"
slice: "s-f07-workspace-timeout"
caused-by: s-f07
failed: true
intent: fix
claimed-by: 3596aec4-c535-4c3b-ab5a-3fd57da75b16
---

# F07 workspace tests did not finish (O3 timeout)

- **caused-by**: s-f07
- **failed oracle**: O3
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=timeout match=yes bytes=82474 at=2026-09-04T15:29:53.364Z
- **O1**: met (`cargo test -p draconic-cli --test bindgen`)
- **O2**: met (`cargo test -p draconic-integration-tests --test bindgen_header`)
- **Roadmap ID**: F07
- **Item**: Bindgen-ish: generate externs from C header subset
- **Tests**: `tests/integration`, `crates/draconic-cli`
- **Targets**: compiler
