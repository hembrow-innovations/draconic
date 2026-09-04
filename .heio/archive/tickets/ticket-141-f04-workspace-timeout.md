---
id: "ticket-141-f04-workspace-timeout"
title: "F04 workspace tests did not finish (O3 timeout)"
kind: ticket
status: closed
labels: feature
tags: []
sprint: platform
created_at: "2026-09-04T15:21:24Z"
updated_at: "2026-09-04T20:50:41.788Z"
slice: "s-f04-workspace-timeout"
caused-by: s-f04
failed: true
intent: fix
claimed-by: 841ba482-50b7-43fa-9de6-ad1afa639ef7
---

# F04 workspace tests did not finish (O3 timeout)

- **caused-by**: s-f04
- **failed oracle**: O3
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=timeout match=yes bytes=79149 at=2026-09-04T15:20:46.230Z
- **O1**: met (`cargo test -p draconic-conformance --test ffi_link_static`)
- **O2**: met (`cargo test -p draconic-integration-tests --test ffi_link_static`)
- **Roadmap ID**: F04
- **Item**: Link external static lib (`.a`); call one symbol
- **Tests**: `tests/conformance` fixtures `ffi/link_static`, `tests/integration`
- **Targets**: native
