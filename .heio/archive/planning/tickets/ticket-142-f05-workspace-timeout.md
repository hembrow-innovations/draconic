---
id: "ticket-142-f05-workspace-timeout"
title: "F05 workspace tests did not finish (O3 timeout)"
kind: ticket
status: closed
ticket_type: feature
labels: feature
tags: []
sprint: "platform"
created_at: "2026-09-04T15:25:37Z"
updated_at: "2026-09-06T18:00:00Z"
slice: "slice-276-f05-workspace-timeout"
---

# F05 workspace tests did not finish (O3 timeout)

- **caused-by**: slice-277-f05
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
