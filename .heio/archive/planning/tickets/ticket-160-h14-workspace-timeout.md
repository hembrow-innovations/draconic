---
id: "ticket-160-h14-workspace-timeout"
title: "H14 workspace tests did not finish (O3 timeout)"
kind: ticket
status: closed
ticket_type: feature
labels: feature
tags: []
sprint: "platform"
created_at: "2026-09-04T17:33:31Z"
updated_at: "2026-09-06T18:00:00Z"
slice: "slice-312-h14-workspace-timeout"
---

# H14 workspace tests did not finish (O3 timeout)

- **caused-by**: slice-313-h14
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
