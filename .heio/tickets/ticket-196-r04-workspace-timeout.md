---
id: "ticket-196-r04-workspace-timeout"
title: "R04 workspace tests did not finish (O3 workspace-timeout)"
kind: ticket
status: ready-for-agent
labels: bug
tags: []
sprint: platform
created_at: "2026-09-05T16:28:23Z"
updated_at: "2026-09-05T16:28:23Z"
caused-by: s-r04
failed: true
intent: fix
---

# R04 workspace tests did not finish (O3 workspace-timeout)

Reviewer miss on [[s-r04]]. This is a budget miss, not a new ROADMAP atom. R04 / R04.01 / R04.02 stay `done` on ROADMAP.md. Next Builder must beat the oracle budget on `cargo test --workspace`, not re-open the language row.

- **caused-by**: s-r04
- **failed oracle**: O3
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=timeout match=yes bytes=109117 at=2026-09-05T16:27:36.834Z
- **O1**: met (`cargo test -p draconic-runtime --lib abort_policy`)
- **O2**: met (`cargo test -p draconic-conformance --test panic_policy`)
- **gap**: O3 matched EXPECT (`test result: ok.`) but the oracle CHECK budget blew (`exit=timeout`). Not a product fail. Not a new R04 Loop atom.
- **beat**: `cargo test --workspace` exits 0 and prints `test result: ok.` inside the oracle CHECK budget. O1/O2 stay green.
