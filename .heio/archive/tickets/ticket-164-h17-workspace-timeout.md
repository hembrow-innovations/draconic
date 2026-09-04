---
id: "ticket-164-h17-workspace-timeout"
title: "H17 workspace tests did not finish (O3 timeout)"
kind: ticket
status: closed
labels: feature
tags: []
sprint: platform
created_at: "2026-09-04T18:01:08Z"
updated_at: "2026-09-04T20:50:41.788Z"
slice: "s-h17-workspace-timeout"
caused-by: s-h17
failed: true
intent: fix
claimed-by: 2a90baba-981f-43ff-aeaa-83974ee3a063
---

# H17 workspace tests did not finish (O3 timeout)

- **caused-by**: s-h17
- **failed oracle**: O3
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=timeout match=yes bytes=83295 at=2026-09-04T18:00:30.722Z
- **O1**: met (`cargo test -p draconic-integration-tests --test http_echo --test host_cutover`)
- **O2**: met (`cargo test -p draconic-integration-tests --test todo_server`)
- **Roadmap ID**: H17
- **Item**: Success Programs & host cutover
- **Tests**: `examples/http-echo`, `examples/todo`
- **Targets**: native
