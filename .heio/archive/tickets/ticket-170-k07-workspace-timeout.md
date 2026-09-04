---
id: "ticket-170-k07-workspace-timeout"
title: "K07 workspace tests did not finish (O3 timeout)"
kind: ticket
status: closed
labels: feature
tags: []
sprint: platform
created_at: "2026-09-04T18:44:49Z"
updated_at: "2026-09-04T20:50:41.788Z"
slice: "s-k07-workspace-timeout"
caused-by: s-k07
failed: true
intent: fix
claimed-by: a4ff7b87-8439-49c2-a476-7f0cccab7d52
---

# K07 workspace tests did not finish (O3 timeout)

- **caused-by**: s-k07
- **failed oracle**: O3
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=timeout match=yes bytes=84353 at=2026-09-04T18:43:47.505Z
- **O1**: met (`cargo test -p draconic-cli --test build`)
- **O2**: met (`cargo test -p draconic-pkg ensure`)
- **Roadmap ID**: K07
- **Item**: Build integration: auto-fetch; `--offline`
- **Tests**: `crates/draconic-cli`, `crates/draconic-pkg`
- **Targets**: compiler
