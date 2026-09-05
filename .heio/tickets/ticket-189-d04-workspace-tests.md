---
id: "ticket-189-d04-workspace-tests"
title: "D04 workspace tests did not pass (O4)"
kind: ticket
status: promoted
labels: bug
tags: []
sprint: platform
created_at: "2026-09-05T01:28:20Z"
updated_at: "2026-09-05T01:29:39Z"
slice: "s-d04-workspace-tests"
caused-by: s-d04-workspace-disabled-gha
failed: true
intent: fix
---

# D04 workspace tests did not pass (O4)

- **caused-by**: s-d04-workspace-disabled-gha
- **failed oracle**: O4
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=101 match=yes bytes=67805 at=2026-09-05T01:27:28.138Z
- **O1**: met (`cargo test -p draconic-integration-tests --test cross_compile`)
- **O2**: met (`cargo test -p draconic-integration-tests --test cross_compile_matrix`)
- **O3**: met (`cargo test -p draconic-integration-tests --test release_artifact`)
- **Note**: D04 / D04.02 / D01.01 workflow readers are green against `.yml.disabled`. Do not restore live `.yml` names. Workspace still exits 101.
