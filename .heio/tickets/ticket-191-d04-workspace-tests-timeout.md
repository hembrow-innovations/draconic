---
id: "ticket-191-d04-workspace-tests-timeout"
title: "D04 workspace tests did not finish (O1 workspace-timeout)"
kind: ticket
status: ready-for-agent
labels: bug
tags: []
sprint: platform
created_at: "2026-09-05T13:09:50Z"
updated_at: "2026-09-05T13:09:50Z"
caused-by: s-d04-workspace-tests
failed: true
intent: fix
---

# D04 workspace tests did not finish (O1 workspace-timeout)

Reviewer miss on [[s-d04-workspace-tests]]. This is a budget miss, not a new ROADMAP atom. D04 / D04.02 / D01.01 stay `done` on ROADMAP.md. Next Builder must beat the oracle budget on `cargo test --workspace`, not re-open the language row.

- **caused-by**: s-d04-workspace-tests
- **failed oracle**: O1
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=timeout match=yes bytes=178334 at=2026-09-05T13:07:19.556Z
- **O2**: met (`cargo test -p draconic-integration-tests --test cross_compile`)
- **O3**: met (`cargo test -p draconic-integration-tests --test cross_compile_matrix`)
- **O4**: met (`cargo test -p draconic-integration-tests --test release_artifact`)
- **gap**: O1 matched EXPECT (`test result: ok.`) but the oracle CHECK budget blew (`exit=timeout`). Not a product fail. Not a new D04 Loop atom. Do not restore live `.yml` names.
- **beat**: `cargo test --workspace` exits 0 and prints `test result: ok.` inside the oracle CHECK budget. O2/O3/O4 stay green against `.yml.disabled`.
