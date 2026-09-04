---
id: "ticket-163-h17-04-workspace-timeout"
title: "H17.04 workspace tests did not finish (O2 timeout)"
kind: ticket
status: closed
labels: feature
tags: []
sprint: platform
created_at: "2026-09-04T17:52:31Z"
updated_at: "2026-09-04T20:50:41.788Z"
slice: "s-h17-04-workspace-timeout"
caused-by: s-h17-04
failed: true
intent: fix
claimed-by: 089afd30-58a5-41d5-a2f2-2a073ed61635
---

# H17.04 workspace tests did not finish (O2 timeout)

- **caused-by**: s-h17-04
- **failed oracle**: O2
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=timeout match=yes bytes=103871 at=2026-09-04T17:52:02.342Z
- **O1**: met (`cargo test -p draconic-conformance --test host_policy`)
- **Roadmap ID**: H17.04
- **Item**: Optional JS/Node bridge for subset host APIs (after native green)
- **Tests**: `tests/conformance` fixtures `host/policy`
- **Targets**: js
