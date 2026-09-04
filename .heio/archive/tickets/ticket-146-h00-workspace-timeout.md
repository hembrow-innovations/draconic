---
id: "ticket-146-h00-workspace-timeout"
title: "H00 workspace tests did not finish (O3 timeout)"
kind: ticket
status: closed
labels: feature
tags: []
sprint: platform
created_at: "2026-09-04T15:44:52Z"
updated_at: "2026-09-04T20:50:41.788Z"
slice: "s-h00-workspace-timeout"
caused-by: s-h00
failed: true
intent: fix
claimed-by: 6926435f-904c-4711-8d07-3572b9a2ebe8
---

# H00 workspace tests did not finish (O3 timeout)

- **caused-by**: s-h00
- **failed oracle**: O3
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=timeout match=yes bytes=72367 at=2026-09-04T15:43:21.456Z
- **O1**: met (`cargo test -p draconic-conformance --test host_policy`)
- **O2**: met (`cargo test -p draconic-runtime --lib`)
- **Roadmap ID**: H00
- **Item**: Host I/O surface policy: module/global shape, error model, js hard-error vs polyfill matrix
- **Tests**: `tests/conformance/host/policy`, `crates/draconic-runtime`
- **Targets**: both
