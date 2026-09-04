---
id: "ticket-168-k04-workspace-timeout"
title: "K04 workspace tests did not finish (O2 timeout)"
kind: ticket
status: closed
labels: feature
tags: []
sprint: platform
created_at: "2026-09-04T18:33:32Z"
updated_at: "2026-09-04T20:50:41.788Z"
slice: "s-k04-workspace-timeout"
caused-by: s-k04
failed: true
intent: fix
claimed-by: 4d5b6444-41fa-4595-a99a-56b7b1536da0
---

# K04 workspace tests did not finish (O2 timeout)

- **caused-by**: s-k04
- **failed oracle**: O2
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=timeout match=yes bytes=66535 at=2026-09-04T18:32:54.318Z
- **O1**: met (`cargo test -p draconic-pkg resolve`)
- **Roadmap ID**: K04
- **Item**: Version resolve: semver tag → commit OID; fail closed
- **Tests**: `crates/draconic-pkg`
- **Targets**: compiler
