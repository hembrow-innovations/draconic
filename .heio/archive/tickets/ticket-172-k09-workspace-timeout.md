---
id: "ticket-172-k09-workspace-timeout"
title: "K09 workspace tests did not finish (O3 timeout)"
kind: ticket
status: closed
labels: feature
tags: []
sprint: platform
created_at: "2026-09-04T18:59:10Z"
updated_at: "2026-09-04T20:50:41.788Z"
slice: "s-k09-workspace-timeout"
caused-by: s-k09
failed: true
intent: fix
claimed-by: 193aac9c-ee3d-42aa-ad04-4fe30820f241
---

# K09 workspace tests did not finish (O3 timeout)

- **caused-by**: s-k09
- **failed oracle**: O3
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=timeout match=yes bytes=103685 at=2026-09-04T18:58:53.924Z
- **O1**: met (`cargo test -p draconic-packages-tests --test k09_01_resolve_fetch`)
- **O2**: met (`cargo test -p draconic-packages-tests --test k09_02_build_consumer`)
- **Roadmap ID**: K09
- **Item**: E2E: temp git dep + consumer Program
- **Tests**: `tests/packages`
- **Targets**: compiler
