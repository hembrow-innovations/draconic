---
id: "ticket-205-l07-02-workspace-tests"
title: "L07.02 workspace tests still exit 101"
kind: ticket
status: closed
ticket_type: bug
labels: bug
tags: []
sprint: "platform"
created_at: "2026-09-05T21:15:00Z"
updated_at: "2026-09-09T09:15:36Z"
---
# L07.02 workspace tests still exit 101

Reviewer miss on [[slice-365-l07-02-workspace-tests-native]]. L07.02 stays `done` on ROADMAP.md. Beat the workspace oracle, not a new language atom.

- **caused-by**: slice-365-l07-02-workspace-tests-native
- **failed oracle**: O1
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=101 match=yes bytes=132849
- **O2**: met (`cargo test -p draconic-conformance --test stdlib_flags`)
- **gap**: workspace CHECK exited 101. EXPECT substring was present from passing crates; the suite was not fully green. Not a new L07.02 Loop atom. Do not restore deleted pi-worktrees.
- **beat**: `cargo test --workspace` exits 0 and prints `test result: ok.` Native runtime C compile still uses tree files when present and embedded sources when the compile-time path is gone.
