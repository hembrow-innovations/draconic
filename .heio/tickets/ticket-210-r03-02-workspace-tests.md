---
id: "ticket-210-r03-02-workspace-tests"
title: "R03.02 workspace tests still exit 101"
kind: ticket
status: ready-for-agent
labels: bug
tags: []
sprint: platform
created_at: "2026-09-05T21:15:00Z"
updated_at: "2026-09-05T21:15:00Z"
caused-by: s-r03-02-workspace-tests
failed: true
intent: fix
---

# R03.02 workspace tests still exit 101

Reviewer miss on [[s-r03-02-workspace-tests]]. R03.02 stays `done` on ROADMAP.md. Beat the workspace oracle, not a new language atom.

- **caused-by**: s-r03-02-workspace-tests
- **failed oracle**: O2
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=101 match=yes bytes=132849
- **O1**: met (`cargo test -p draconic-integration-tests --test supply_chain_lock_hash_mismatch`)
- **gap**: workspace CHECK exited 101 (fail, not timeout). First pass timed out with match=yes; a longer reverify then exited 101. EXPECT substring was present; the suite was not fully green. Not a new R03.02 Loop atom.
- **beat**: `cargo test --workspace` exits 0 and prints `test result: ok.` supply_chain_lock_hash_mismatch stays green.
