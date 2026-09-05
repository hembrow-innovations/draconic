---
id: "ticket-198-r05-02-workspace-tests"
title: "R05.02 workspace tests did not pass (O3)"
kind: ticket
status: ready-for-agent
labels: bug
tags: []
sprint: platform
created_at: "2026-09-05T17:31:38Z"
updated_at: "2026-09-05T17:31:38Z"
caused-by: s-r05-02
failed: true
intent: fix
---

# R05.02 workspace tests did not pass (O3)

Reviewer miss on [[s-r05-02]]. Not a new ROADMAP atom. R05.02 stays `done` on ROADMAP.md. Next Builder must beat the failed workspace oracle, not re-open the language row.

- **caused-by**: s-r05-02
- **failed oracle**: O3
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=101 match=yes bytes=137759 at=2026-09-05T17:30:10.952Z
- **O1**: met (`cargo test -p draconic-runtime --lib fuzz`)
- **O2**: met (`cargo test -p draconic-embed --lib fuzz`)
- **gap**: O3 printed EXPECT (`test result: ok.`) in crate output but `cargo test --workspace` exited 101. Product fail, not an oracle-budget timeout. Not a new R05.02 Loop atom.
- **beat**: `cargo test --workspace` exits 0 and prints `test result: ok.` O1 and O2 stay green.
- **Roadmap ID**: R05.02 (already `done`; do not mint a new language atom)
- **Item**: Embed/runtime fuzz or stress hooks
- **Tests**: `crates/draconic-runtime` fuzz, `crates/draconic-embed`
- **Targets**: native
