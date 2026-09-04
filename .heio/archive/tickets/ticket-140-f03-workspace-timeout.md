---
id: "ticket-140-f03-workspace-timeout"
title: "F03 workspace tests did not finish (O2 timeout)"
kind: ticket
status: closed
labels: feature
tags: []
sprint: platform
slice: "s-f03-workspace-timeout"
created_at: "2026-09-04T15:17:26Z"
updated_at: "2026-09-04T20:50:41.788Z"
caused-by: s-f03
failed: true
intent: fix
claimed-by: e1b4f4cd-501c-4c3c-ad82-9edf98496006
---

# F03 workspace tests did not finish (O2 timeout)

- **caused-by**: s-f03
- **failed oracle**: O2
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=timeout match=yes bytes=73653 at=2026-09-04T15:16:42.418Z
- **O1**: met (`cargo test -p draconic-conformance --test ffi_layout`)
- **Roadmap ID**: F03
- **Item**: C-compatible struct layout (repr(C)-style); read/write both sides
- **Tests**: `tests/conformance` fixtures `ffi/layout`
- **Targets**: native
