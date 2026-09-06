---
id: "ticket-209-p04-workspace-tests"
title: "P04 workspace tests still exit 101"
kind: ticket
status: promoted
labels: bug
tags: []
sprint: platform
created_at: "2026-09-05T21:15:00Z"
updated_at: "2026-09-06T04:39:34Z"
caused-by: s-p04-workspace-tests
failed: true
intent: fix
---

# P04 workspace tests still exit 101

Reviewer miss on [[s-p04-workspace-tests]]. P04 stays `done` on ROADMAP.md. Beat the workspace oracle, not a new language atom.

- **caused-by**: s-p04-workspace-tests
- **failed oracle**: O2
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=101 match=yes bytes=132850
- **O1**: met (`cargo test -p draconic-integration-tests --test flagship_service`)
- **gap**: workspace CHECK exited 101 (fail, not timeout). EXPECT substring was present from passing crates; the suite was not fully green. Not a new P04 Loop atom.
- **beat**: `cargo test --workspace` exits 0 and prints `test result: ok.` flagship_service stays green.
