---
id: "ticket-138-e18-44-workspace-timeout"
title: "E18.44 remainder workspace tests did not finish (O2 timeout)"
kind: ticket
status: closed
labels: feature
tags: []
sprint: platform
slice: "s-e18-44-workspace-timeout"
created_at: "2026-09-04T15:08:39Z"
updated_at: "2026-09-04T20:50:41.788Z"
caused-by: s-e18-44
failed: true
intent: fix
claimed-by: 5cbd746d-5348-4bf3-9c0a-1d5cd32e9069
---

# E18.44 remainder workspace tests did not finish (O2 timeout)

- **caused-by**: s-e18-44
- **failed oracle**: O2
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=timeout match=yes bytes=108629 at=2026-09-04T15:07:54.950Z
- **O1**: met (`cargo test -p draconic-conformance --test annex_b`)
- **Roadmap ID**: E18.44
- **Item**: Untracked ECMA-262 remainder beyond E01–E18 children (file finer rows as discovered; do not drop)
- **Tests**: `tests/conformance` (new fixtures as filed); typically `tests/conformance/fixtures/es/annex-b`
- **Targets**: js
