---
id: "ticket-181-l03-workspace-timeout"
title: "L03 workspace tests did not finish (O2 timeout)"
kind: ticket
status: closed
ticket_type: feature
labels: feature
tags: []
sprint: "platform"
created_at: "2026-09-04T19:57:13Z"
updated_at: "2026-09-06T18:00:00Z"
slice: "slice-356-l03-workspace-timeout"
---

# L03 workspace tests did not finish (O2 timeout)

- **caused-by**: slice-357-l03
- **failed oracle**: O2
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=timeout match=yes bytes=76260 at=2026-09-04T19:56:21.429Z
- **O1**: met (`cargo test -p draconic-conformance --test stdlib_crypto`)
- **Roadmap ID**: L03
- **Item**: Crypto: SHA-256 digest + secure random bytes
- **Tests**: `tests/conformance` fixtures `stdlib/crypto`
- **Targets**: both
