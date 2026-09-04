---
id: "ticket-124-c05-workspace-timeout"
title: "C05 workspace tests did not finish (O3 timeout)"
kind: ticket
status: closed
labels: feature
tags: []
sprint: platform
created_at: "2026-09-04T13:43:34Z"
updated_at: "2026-09-04T20:50:41.788Z"
slice: "s-c05-workspace-timeout"
caused-by: s-c05
failed: true
intent: fix
claimed-by: bb3a825e-2415-4120-a3d1-402851bb0f0f
---

# C05 workspace tests did not finish (O3 timeout)

- **caused-by**: s-c05
- **failed oracle**: O3
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=timeout match=yes bytes=79421 at=2026-09-04T13:42:49.907Z
- **O1**: met (`cargo test -p draconic-conformance --test concurrency_cancel`)
- **O2**: met (`cargo test -p draconic-runtime --lib`)
- **Roadmap ID**: C05
- **Item**: Structured cancellation / timeout helpers on async work (channels + timers)
- **Tests**: `tests/conformance` fixtures `concurrency/cancel`
- **Targets**: both
