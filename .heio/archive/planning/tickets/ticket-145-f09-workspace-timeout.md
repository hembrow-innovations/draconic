---
id: "ticket-145-f09-workspace-timeout"
title: "F09 workspace tests did not finish (O3 timeout)"
kind: ticket
status: closed
ticket_type: feature
labels: feature
tags: []
sprint: "platform"
created_at: "2026-09-04T15:39:35Z"
updated_at: "2026-09-06T18:00:00Z"
slice: "slice-282-f09-workspace-timeout"
---

# F09 workspace tests did not finish (O3 timeout)

- **caused-by**: slice-283-f09
- **failed oracle**: O3
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=timeout match=yes bytes=64586 at=2026-09-04T15:38:32.761Z
- **O1**: met (`cargo test -p draconic-backend-llvm wasm32_wasi`)
- **O2**: met (`cargo test -p draconic-integration-tests --test wasm32_wasi`)
- **Roadmap ID**: F09
- **Item**: Optional later: wasm32/wasi emit + link smoke
- **Tests**: `tests/integration`, `crates/draconic-backend-llvm`
- **Targets**: native
