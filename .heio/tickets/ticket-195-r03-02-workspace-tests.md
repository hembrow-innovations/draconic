---
id: "ticket-195-r03-02-workspace-tests"
title: "R03.02 workspace tests did not pass (O2)"
kind: ticket
status: ready-for-agent
labels: bug
tags: []
sprint: platform
created_at: "2026-09-05T15:55:50Z"
updated_at: "2026-09-05T15:55:50Z"
caused-by: s-r03-02
failed: true
intent: fix
---

# R03.02 workspace tests did not pass (O2)

Reviewer miss on [[s-r03-02]]. Not a new ROADMAP atom. R03.02 stays `done` on ROADMAP.md. Next Builder must beat the failed workspace oracle, not re-open the language row.

- **caused-by**: s-r03-02
- **failed oracle**: O2
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=101 match=yes bytes=113668 at=2026-09-05T15:54:40.560Z
- **O1**: met (`cargo test -p draconic-integration-tests --test supply_chain_lock_hash_mismatch`)
- **gap**: O2 printed EXPECT (`test result: ok.`) in crate output but `cargo test --workspace` exited 101. Product fail, not an oracle-budget timeout. Not a new R03.02 Loop atom.
- **beat**: `cargo test --workspace` exits 0 and prints `test result: ok.` O1 stays green.
- **Roadmap ID**: R03.02 (already `done`; do not mint a new language atom)
- **Item**: Integration: lock hash mismatch hard-fails build
- **Tests**: `tests/integration`, **K08**
- **Targets**: compiler
