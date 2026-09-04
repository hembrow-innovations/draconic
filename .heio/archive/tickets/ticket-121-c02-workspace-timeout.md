---
id: "ticket-121-c02-workspace-timeout"
title: "C02 workspace tests did not finish (O2 timeout)"
kind: ticket
status: closed
labels: feature
tags: []
sprint: platform
slice: "s-c02-workspace-timeout"
created_at: "2026-09-04T13:29:15Z"
updated_at: "2026-09-04T20:50:41.788Z"
caused-by: s-c02
failed: true
intent: fix
claimed-by: 87caf3a1-884f-4f0b-b9e7-707dce027915
---

# C02 workspace tests did not finish (O2 timeout)

- **caused-by**: s-c02
- **failed oracle**: O2
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=timeout match=yes bytes=72781 at=2026-09-04T13:28:38.782Z
- **O1**: met (`cargo test -p draconic-conformance --test concurrency_channels`)
- **Roadmap ID**: C02
- **Item**: Message-passing channels: send/recv; structured-clone or transfer policy; bounded buffer as designed
- **Tests**: `tests/conformance` fixtures `concurrency/channels`
- **Targets**: both
