---
id: "ticket-160-h14-workspace-timeout"
title: "H14 workspace tests did not finish (O3 timeout)"
kind: ticket
status: closed
labels: feature
tags: []
sprint: platform
created_at: "2026-09-04T17:33:31Z"
updated_at: "2026-09-04T20:50:41.788Z"
slice: "s-h14-workspace-timeout"
caused-by: s-h14
failed: true
intent: fix
claimed-by: 3487d882-8d29-4619-8169-f521e62cb02b
---

# H14 workspace tests did not finish (O3 timeout)

- **caused-by**: s-h14
- **failed oracle**: O3
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=timeout match=yes bytes=105062 at=2026-09-04T17:32:57.859Z
- **O1**: met (`cargo test -p draconic-conformance --test host_process signal`)
- **O2**: met (`cargo test -p draconic-runtime host_signal`)
- **Roadmap ID**: H14
- **Item**: Signals
- **Tests**: `tests/conformance/host/process/signals`
- **Targets**: native
