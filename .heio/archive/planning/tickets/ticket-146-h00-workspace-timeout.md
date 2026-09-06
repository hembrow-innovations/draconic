---
id: "ticket-146-h00-workspace-timeout"
title: "H00 workspace tests did not finish (O3 timeout)"
kind: ticket
status: closed
ticket_type: feature
labels: feature
tags: []
sprint: "platform"
created_at: "2026-09-04T15:44:52Z"
updated_at: "2026-09-06T18:00:00Z"
slice: "slice-284-h00-workspace-timeout"
---

# H00 workspace tests did not finish (O3 timeout)

- **caused-by**: slice-285-h00
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
