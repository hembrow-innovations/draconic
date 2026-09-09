---
id: "ticket-207-l10-02-workspace-tests"
title: "L10.02 workspace tests still exit 101"
kind: ticket
status: closed
ticket_type: bug
labels: bug
tags: []
sprint: "platform"
created_at: "2026-09-05T21:15:00Z"
updated_at: "2026-09-09T09:15:36Z"
---
# L10.02 workspace tests still exit 101

Reviewer miss on [[slice-375-l10-02-workspace-timeout]]. L10.02 stays `done` on ROADMAP.md. Beat the workspace oracle, not a new language atom.

- **caused-by**: slice-375-l10-02-workspace-timeout
- **failed oracle**: O2
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=101 match=yes bytes=42064
- **O1**: met (`cargo test -p draconic-conformance --test stdlib_crypto`)
- **gap**: workspace CHECK exited 101 (fail, not timeout). EXPECT substring was present; cargo failed. Not a new L10.02 Loop atom.
- **beat**: `cargo test --workspace` exits 0 and prints `test result: ok.` stdlib_crypto stays green.
