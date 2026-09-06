---
id: "ticket-140-f03-workspace-timeout"
title: "F03 workspace tests did not finish (O2 timeout)"
kind: ticket
status: closed
ticket_type: feature
labels: feature
tags: []
sprint: "platform"
slice: "slice-272-f03-workspace-timeout"
created_at: "2026-09-04T15:17:26Z"
updated_at: "2026-09-06T18:00:00Z"
---

# F03 workspace tests did not finish (O2 timeout)

- **caused-by**: slice-273-f03
- **failed oracle**: O2
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=timeout match=yes bytes=73653 at=2026-09-04T15:16:42.418Z
- **O1**: met (`cargo test -p draconic-conformance --test ffi_layout`)
- **Roadmap ID**: F03
- **Item**: C-compatible struct layout (repr(C)-style); read/write both sides
- **Tests**: `tests/conformance` fixtures `ffi/layout`
- **Targets**: native
