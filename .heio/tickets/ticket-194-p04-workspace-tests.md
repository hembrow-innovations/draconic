---
id: "ticket-194-p04-workspace-tests"
title: "P04 workspace tests did not pass (O2)"
kind: ticket
status: ready-for-agent
labels: bug
tags: []
sprint: platform
created_at: "2026-09-05T14:25:08Z"
updated_at: "2026-09-05T14:25:08Z"
caused-by: s-p04
failed: true
intent: fix
---

# P04 workspace tests did not pass (O2)

Reviewer miss on [[s-p04]]. Not a new ROADMAP atom. P04 stays `done` on ROADMAP.md. Next Builder must beat the failed workspace oracle, not re-open the language row.

- **caused-by**: s-p04
- **failed oracle**: O2
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=101 match=yes bytes=93257 at=2026-09-05T14:22:54.916Z
- **O1**: met (`cargo test -p draconic-integration-tests --test flagship_service`)
- **gap**: O2 printed EXPECT (`test result: ok.`) in crate output but `cargo test --workspace` exited 101. Product fail, not an oracle-budget timeout. Not a new P04 Loop atom.
- **beat**: `cargo test --workspace` exits 0 and prints `test result: ok.` O1 stays green.
- **Roadmap ID**: P04 (already `done`; do not mint a new language atom)
- **Item**: Flagship service example: typed HTTP + fs/config + git dep (after H17 + K09)
- **Tests**: `examples/`, `tests/integration/tests/flagship_service.rs`
- **Targets**: both
