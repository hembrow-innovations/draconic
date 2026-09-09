---
id: "ticket-213-r05-02-workspace-tests"
title: "R05.02 workspace tests still exit 101"
kind: ticket
status: closed
ticket_type: bug
labels: bug
tags: []
sprint: "platform"
created_at: "2026-09-05T21:15:00Z"
updated_at: "2026-09-09T09:15:36Z"
---
# R05.02 workspace tests still exit 101

Reviewer miss on [[slice-395-r05-02-workspace-tests]]. R05.02 stays `done` on ROADMAP.md. Beat the workspace oracle, not a new language atom.

- **caused-by**: slice-395-r05-02-workspace-tests
- **failed oracle**: O3
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=101 match=yes bytes=132795
- **O1**: met (draconic-runtime fuzz)
- **O2**: met (draconic-embed fuzz)
- **gap**: workspace CHECK exited 101 (fail, not timeout). EXPECT substring was present; the suite was not fully green. Not a new R05.02 Loop atom.
- **beat**: `cargo test --workspace` exits 0 and prints `test result: ok.` runtime fuzz and embed fuzz stay green.
