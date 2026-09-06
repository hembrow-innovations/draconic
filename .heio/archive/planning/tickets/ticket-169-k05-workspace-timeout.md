---
id: "ticket-169-k05-workspace-timeout"
title: "K05 workspace tests did not finish (O3 timeout)"
kind: ticket
status: closed
ticket_type: feature
labels: feature
tags: []
sprint: "platform"
created_at: "2026-09-04T18:37:57Z"
updated_at: "2026-09-06T18:00:00Z"
slice: "slice-330-k05-workspace-timeout"
---

# K05 workspace tests did not finish (O3 timeout)

- **caused-by**: slice-331-k05
- **failed oracle**: O3
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=timeout match=yes bytes=80707 at=2026-09-04T18:37:08.336Z
- **O1**: met (`cargo test -p draconic-cli --test get`)
- **O2**: met (`cargo test -p draconic-cli --test mod_tidy`)
- **Roadmap ID**: K05
- **Item**: CLI: `draconic get`, `draconic mod tidy`
- **Tests**: `crates/draconic-cli`
- **Targets**: compiler
