---
id: "ticket-166-k02-workspace-timeout"
title: "K02 workspace tests did not finish (O2 timeout)"
kind: ticket
status: closed
labels: feature
tags: []
sprint: platform
created_at: "2026-09-04T18:17:04Z"
updated_at: "2026-09-04T20:50:41.788Z"
slice: "s-k02-workspace-timeout"
caused-by: s-k02
failed: true
intent: fix
claimed-by: 8ac51b0b-cd8f-4994-a594-959a58f0c173
---

# K02 workspace tests did not finish (O2 timeout)

- **caused-by**: s-k02
- **failed oracle**: O2
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=timeout match=yes bytes=77246 at=2026-09-04T18:16:29.654Z
- **O1**: met (`cargo test -p draconic-pkg lock`)
- **Roadmap ID**: K02
- **Item**: Lockfile (`draconic.lock`): resolved pins
- **Tests**: `crates/draconic-pkg`
- **Targets**: compiler
