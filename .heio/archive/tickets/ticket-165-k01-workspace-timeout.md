---
id: "ticket-165-k01-workspace-timeout"
title: "K01 workspace tests did not finish (O2 timeout)"
kind: ticket
status: closed
labels: feature
tags: []
sprint: platform
created_at: "2026-09-04T18:10:23Z"
updated_at: "2026-09-04T20:50:41.788Z"
slice: "s-k01-workspace-timeout"
caused-by: s-k01
failed: true
intent: fix
claimed-by: c8b7f82b-b0ef-45d7-a678-630d9ba02338
---

# K01 workspace tests did not finish (O2 timeout)

- **caused-by**: s-k01
- **failed oracle**: O2
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=timeout match=yes bytes=77564 at=2026-09-04T18:09:38.570Z
- **O1**: met (`cargo test -p draconic-pkg --lib`)
- **Roadmap ID**: K01
- **Item**: Manifest (`draconic.toml`): module path, deps, optional path→git URL map
- **Tests**: `crates/draconic-pkg`
- **Targets**: compiler
