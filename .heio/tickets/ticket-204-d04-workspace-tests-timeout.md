---
id: "ticket-204-d04-workspace-tests-timeout"
title: "D04 workspace tests timeout and release_artifact fail"
kind: ticket
status: ready-for-agent
labels: bug
tags: []
sprint: platform
created_at: "2026-09-05T21:15:00Z"
updated_at: "2026-09-05T21:15:00Z"
caused-by: s-d04-workspace-tests-timeout
failed: true
intent: fix
---

# D04 workspace tests timeout and release_artifact fail

Reviewer miss on [[s-d04-workspace-tests-timeout]]. O1 is a budget miss, not a new ROADMAP atom. O4 is a product fail on `release_artifact`. D04 / D04.02 / D01.01 stay `done` on ROADMAP.md.

- **caused-by**: s-d04-workspace-tests-timeout
- **failed oracles**: O1, O4
- **O1 CHECK**: cargo test --workspace
- **O1 EVIDENCE**: unmet exit=timeout match=yes bytes=90371
- **O4 CHECK**: cargo test -p draconic-integration-tests --test release_artifact
- **O4 EVIDENCE**: unmet exit=101 match=no bytes=3329
- **O2**: met (cross_compile)
- **O3**: met (cross_compile_matrix)
- **gap**: O1 matched EXPECT then blew the 600s CHECK budget. O4 did not match EXPECT and exited 101. Not a new D04 Loop atom. Do not restore live `.yml` names.
- **beat**: `cargo test --workspace` exits 0 and prints `test result: ok.` inside the oracle CHECK budget. `release_artifact` stays green against `.yml.disabled`.
