---
id: "ticket-122-c03-workspace-timeout"
title: "C03 workspace tests did not finish (O3 timeout)"
kind: ticket
status: closed
ticket_type: feature
labels: feature
tags: []
sprint: "platform"
slice: "slice-231-c03-workspace-timeout"
created_at: "2026-09-04T13:33:35Z"
updated_at: "2026-09-06T18:00:00Z"
---

# C03 workspace tests did not finish (O3 timeout)

- **caused-by**: slice-232-c03
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
