---
id: "ticket-149-h03-workspace-timeout"
title: "H03 workspace tests did not finish (O2 timeout)"
kind: ticket
status: closed
labels: feature
tags: []
sprint: platform
created_at: "2026-09-04T16:10:02Z"
updated_at: "2026-09-04T20:50:41.788Z"
slice: "s-h03-workspace-timeout"
caused-by: s-h03
failed: true
intent: fix
claimed-by: 9a3adc17-0eac-473e-a5c6-87159078a7ef
---

# H03 workspace tests did not finish (O2 timeout)

- **caused-by**: s-h03
- **failed oracle**: O2
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=timeout match=yes bytes=79321 at=2026-09-04T16:09:39.507Z
- **O1**: met (`cargo test -p draconic-conformance --test host_path`)
- **Roadmap ID**: H03
- **Item**: Path helpers (string ops; no I/O)
- **Tests**: `tests/conformance/host/path`, `crates/draconic-backend-llvm`
- **Targets**: both
