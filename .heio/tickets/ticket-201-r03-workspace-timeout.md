---
id: "ticket-201-r03-workspace-timeout"
title: "R03 workspace tests did not finish (O2 workspace-timeout)"
kind: ticket
status: ready-for-agent
labels: bug
tags: []
sprint: platform
created_at: "2026-09-05T18:34:42Z"
updated_at: "2026-09-05T18:34:42Z"
caused-by: s-r03
failed: true
intent: fix
---

# R03 workspace tests did not finish (O2 workspace-timeout)

Reviewer miss on [[s-r03]]. This is a budget miss, not a new ROADMAP atom. R03 stays on ROADMAP.md as the Builder left it. Next Builder must beat the oracle budget on `cargo test --workspace`, not re-open the language row.

- **caused-by**: s-r03
- **failed oracle**: O2
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=timeout match=yes bytes=192184 at=2026-09-05T18:33:38.960Z
- **O1**: met (`cargo test -p draconic-integration-tests --test supply_chain`)
- **gap**: O2 matched EXPECT (`test result: ok.`) but the oracle CHECK budget blew (`exit=timeout`). Not a product fail. Not a new R03 Loop atom.
- **beat**: `cargo test --workspace` exits 0 and prints `test result: ok.` inside the oracle CHECK budget. O1 stays green.
- **Roadmap ID**: R03 (do not mint a new language atom)
- **Item**: Supply-chain policy tests once **K08** lands (lock verify refuse tamper)
- **Tests**: `tests/integration`, **K08**
- **Targets**: compiler
