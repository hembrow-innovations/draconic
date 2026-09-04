---
id: "ticket-144-f08-workspace-timeout"
title: "F08 workspace tests did not finish (O2 timeout)"
kind: ticket
status: closed
labels: feature
tags: []
sprint: platform
created_at: "2026-09-04T15:35:23Z"
updated_at: "2026-09-04T20:50:41.788Z"
slice: "s-f08-workspace-timeout"
caused-by: s-f08
failed: true
intent: fix
claimed-by: e6c8ebca-559d-4dd4-bc31-45b2c374be09
---

# F08 workspace tests did not finish (O2 timeout)

- **caused-by**: s-f08
- **failed oracle**: O2
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=timeout match=yes bytes=84042 at=2026-09-04T15:34:17.099Z
- **O1**: met (`cargo test -p draconic-conformance --test ffi_policy`)
- **Roadmap ID**: F08
- **Item**: Unsafe/native-only FFI diagnostics; JS hard-error; clear spans
- **Tests**: `tests/conformance` fixtures `ffi/policy`
- **Targets**: both
