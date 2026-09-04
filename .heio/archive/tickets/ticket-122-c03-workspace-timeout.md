---
id: "ticket-122-c03-workspace-timeout"
title: "C03 workspace tests did not finish (O3 timeout)"
kind: ticket
status: closed
labels: feature
tags: []
sprint: platform
slice: "s-c03-workspace-timeout"
created_at: "2026-09-04T13:33:35Z"
updated_at: "2026-09-04T20:50:41.788Z"
caused-by: s-c03
failed: true
intent: fix
claimed-by: a9c62418-2145-40eb-aaec-3585b60519c6
---

# C03 workspace tests did not finish (O3 timeout)

- **caused-by**: s-c03
- **failed oracle**: O3
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=timeout match=yes bytes=83706 at=2026-09-04T13:32:31.803Z
- **O1**: met (`cargo test -p draconic-conformance --test concurrency_sync`)
- **O2**: met (`cargo test -p draconic-runtime --lib`)
- **Roadmap ID**: C03
- **Item**: `once` / thread-safe init; mutex only if Runtime internals need it
- **Tests**: `crates/draconic-runtime`, `tests/conformance` fixtures `concurrency/sync`
- **Targets**: native
