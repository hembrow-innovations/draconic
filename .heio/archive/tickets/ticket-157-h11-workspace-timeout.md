---
id: "ticket-157-h11-workspace-timeout"
title: "H11 workspace tests did not finish (O2 timeout)"
kind: ticket
status: closed
labels: feature
tags: []
sprint: platform
created_at: "2026-09-04T17:17:22Z"
updated_at: "2026-09-04T20:50:41.788Z"
slice: "s-h11-workspace-timeout"
caused-by: s-h11
failed: true
intent: fix
claimed-by: 6e7ab97c-bfb2-4476-ad89-3db10627df2f
---

# H11 workspace tests did not finish (O2 timeout)

- **caused-by**: s-h11
- **failed oracle**: O2
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=timeout match=yes bytes=76727 at=2026-09-04T17:16:37.196Z
- **O1**: met (`cargo test -p draconic-conformance --test host_tls`)
- **Roadmap ID**: H11
- **Item**: TLS
- **Tests**: `tests/conformance/host/net/tls`
- **Targets**: native
