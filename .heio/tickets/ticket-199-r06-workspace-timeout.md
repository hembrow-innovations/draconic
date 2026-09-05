---
id: "ticket-199-r06-workspace-timeout"
title: "R06 workspace tests did not finish (O3 workspace-timeout)"
kind: ticket
status: ready-for-agent
labels: bug
tags: []
sprint: platform
created_at: "2026-09-05T17:58:37Z"
updated_at: "2026-09-05T17:58:37Z"
caused-by: s-r06
failed: true
intent: fix
---

# R06 workspace tests did not finish (O3 workspace-timeout)

Reviewer miss on [[s-r06]]. This is a budget miss, not a new ROADMAP atom. R06 stays `done` on ROADMAP.md. Next Builder must beat the oracle budget on `cargo test --workspace`, not re-open the language row.

- **caused-by**: s-r06
- **failed oracle**: O3
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=timeout match=yes bytes=79966 at=2026-09-05T17:57:20.889Z
- **O1**: met (`cargo test -p draconic-runtime --lib backtrace`)
- **O2**: met (`cargo test -p draconic-integration-tests --test panic_backtrace`)
- **gap**: O3 matched EXPECT (`test result: ok.`) but the oracle CHECK budget blew (`exit=timeout`). Not a product fail. Not a new R06 Loop atom.
- **beat**: `cargo test --workspace` exits 0 and prints `test result: ok.` inside the oracle CHECK budget. O1/O2 stay green.
