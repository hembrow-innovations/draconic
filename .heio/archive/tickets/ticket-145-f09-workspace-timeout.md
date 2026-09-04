---
id: "ticket-145-f09-workspace-timeout"
title: "F09 workspace tests did not finish (O3 timeout)"
kind: ticket
status: closed
labels: feature
tags: []
sprint: platform
created_at: "2026-09-04T15:39:35Z"
updated_at: "2026-09-04T20:50:41.788Z"
slice: "s-f09-workspace-timeout"
caused-by: s-f09
failed: true
intent: fix
claimed-by: 0e03984c-7dd0-4229-ac97-c1d48f22e367
---

# F09 workspace tests did not finish (O3 timeout)

- **caused-by**: s-f09
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
