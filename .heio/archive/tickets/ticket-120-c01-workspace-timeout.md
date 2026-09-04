---
id: "ticket-120-c01-workspace-timeout"
title: "C01 workspace tests did not finish (O2 timeout)"
kind: ticket
status: closed
labels: feature
tags: []
sprint: platform
slice: "s-c01-workspace-timeout"
created_at: "2026-09-04T13:24:50Z"
updated_at: "2026-09-04T20:50:41.788Z"
caused-by: s-c01
failed: true
intent: fix
claimed-by: 5a5460da-abbe-4dc3-ba2f-73891426b21c
---

# C01 workspace tests did not finish (O2 timeout)

- **caused-by**: s-c01
- **failed oracle**: O2
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=timeout match=yes bytes=67669 at=2026-09-04T13:23:18.751Z
- **O1**: met (`cargo test -p draconic-conformance --test concurrency_workers`)
- **Roadmap ID**: C01
- **Item**: Worker / OS thread: spawn isolate running module/fn; join/terminate; no shared JS heap by default
- **Tests**: `tests/conformance` fixtures `concurrency/workers`
- **Targets**: both
