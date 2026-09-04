---
id: "ticket-142-f05-workspace-timeout"
title: "F05 workspace tests did not finish (O3 timeout)"
kind: ticket
status: closed
labels: feature
tags: []
sprint: platform
created_at: "2026-09-04T15:25:37Z"
updated_at: "2026-09-04T20:50:41.788Z"
slice: "s-f05-workspace-timeout"
caused-by: s-f05
failed: true
intent: fix
claimed-by: 20fea2de-6ef8-442e-86a5-8eba4fd2ce7e
---

# F05 workspace tests did not finish (O3 timeout)

- **caused-by**: s-f05
- **failed oracle**: O3
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=timeout match=yes bytes=104363 at=2026-09-04T15:25:03.719Z
- **O1**: met (`cargo test -p draconic-conformance --test ffi_link_dynamic`)
- **O2**: met (`cargo test -p draconic-integration-tests --test ffi_link_dynamic`)
- **Roadmap ID**: F05
- **Item**: Link/load dynamic lib (`.so`/`.dylib`/`.dll`); call one symbol
- **Tests**: `tests/conformance` fixtures `ffi/link_dynamic`, `tests/integration`
- **Targets**: native
