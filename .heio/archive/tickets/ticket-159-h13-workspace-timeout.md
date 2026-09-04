---
id: "ticket-159-h13-workspace-timeout"
title: "H13 workspace tests did not finish (O2 timeout)"
kind: ticket
status: closed
labels: feature
tags: []
sprint: platform
created_at: "2026-09-04T17:29:33Z"
updated_at: "2026-09-04T20:50:41.788Z"
slice: "s-h13-workspace-timeout"
caused-by: s-h13
failed: true
intent: fix
claimed-by: 08c5dbc1-e1f0-4375-9075-b7c600fda05b
---

# H13 workspace tests did not finish (O2 timeout)

- **caused-by**: s-h13
- **failed oracle**: O2
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=timeout match=yes bytes=85395 at=2026-09-04T17:28:50.495Z
- **O1**: met (`cargo test -p draconic-conformance --test host_http2`)
- **Roadmap ID**: H13
- **Item**: HTTP/2 (later; not v1 bar)
- **Tests**: `tests/conformance/host/http2`
- **Targets**: native
