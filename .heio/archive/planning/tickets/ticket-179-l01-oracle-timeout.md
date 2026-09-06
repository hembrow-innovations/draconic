---
id: "ticket-179-l01-oracle-timeout"
title: "L01 encoding and workspace checks did not finish (O1 O2 timeout)"
kind: ticket
status: closed
ticket_type: feature
labels: feature
tags: []
sprint: "platform"
created_at: "2026-09-04T19:43:49Z"
updated_at: "2026-09-06T18:00:00Z"
slice: "slice-350-l01-oracle-timeout"
---

# L01 encoding and workspace checks did not finish (O1 O2 timeout)

- **caused-by**: slice-351-l01
- **failed oracle**: O1 O2
- **CHECK O1**: cargo test -p draconic-conformance --test encoding
- **CHECK O2**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE O1**: unmet exit=timeout match=no bytes=2674 at=2026-09-04T19:40:32.175Z
- **EVIDENCE O2**: unmet exit=timeout match=yes bytes=6614 at=2026-09-04T19:42:32.180Z
- **Roadmap ID**: L01
- **Item**: Encoding: UTF-8 bytes↔string, Base64, hex
- **Tests**: `tests/conformance` fixtures `stdlib/encoding`
- **Targets**: both
