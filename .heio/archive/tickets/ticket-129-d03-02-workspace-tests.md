---
id: "ticket-129-d03-02-workspace-tests"
title: "D03.02 workspace tests did not pass (O2)"
kind: ticket
status: closed
labels: feature
tags: []
sprint: platform
created_at: "2026-09-04T14:18:05Z"
updated_at: "2026-09-04T20:50:41.788Z"
slice: "s-d03-02-workspace-tests"
caused-by: s-d03-02
failed: true
intent: fix
claimed-by: d208dfba-5643-43d4-bec8-2fafaf62fda4
---

# D03.02 workspace tests did not pass (O2)

- **caused-by**: s-d03-02
- **failed oracle**: O2
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=101 match=no bytes=6625 at=2026-09-04T14:16:37.928Z
- **O1**: met (`cargo test -p draconic-integration-tests --test reproducible_emit`)
- **observed**: `cargo test --workspace` did not compile `draconic-pkg` (lib test); `LaterPackaging` undeclared in `crates/draconic-pkg/src/later.rs`
- **Roadmap ID**: D03.02
- **Item**: Same source + pin → byte-identical or documented-equivalent emit
- **Tests**: `tests/integration`
- **Targets**: compiler
