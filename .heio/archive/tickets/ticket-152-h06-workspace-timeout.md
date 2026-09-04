---
id: "ticket-152-h06-workspace-timeout"
title: "H06 workspace tests did not finish (O2 timeout)"
kind: ticket
status: closed
labels: feature
tags: []
sprint: platform
created_at: "2026-09-04T16:36:19Z"
updated_at: "2026-09-04T20:50:41.788Z"
slice: "s-h06-workspace-timeout"
caused-by: s-h06
failed: true
intent: fix
claimed-by: 93f0224e-0ca4-41fb-ac17-1c9dc3dfe234
---

# H06 workspace tests did not finish (O2 timeout)

- **caused-by**: s-h06
- **failed oracle**: O2
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=timeout match=yes bytes=77797 at=2026-09-04T16:35:55.904Z
- **O1**: met (`cargo test -p draconic-conformance --test host_tcp`)
- **Roadmap ID**: H06
- **Item**: TCP sockets (sockets-first)
- **Tests**: `tests/conformance/host/net/tcp`, `crates/draconic-backend-llvm`, `crates/draconic-runtime`
- **Targets**: native
