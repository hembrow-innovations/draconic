---
id: "ticket-135-d05-02-workspace-timeout"
title: "D05.02 workspace tests did not finish (O3 timeout)"
kind: ticket
status: closed
ticket_type: feature
labels: feature
tags: []
sprint: "platform"
created_at: "2026-09-04T14:47:41Z"
updated_at: "2026-09-06T18:00:00Z"
slice: "slice-260-d05-02-workspace-timeout"
---

# D05.02 workspace tests did not finish (O3 timeout)

- **caused-by**: slice-261-d05-02
- **failed oracle**: O3
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=timeout match=yes bytes=82237 at=2026-09-04T14:47:12.626Z
- **O1**: met (`cargo test -p draconic-cli --test lto_flag`)
- **O2**: met (`cargo test -p draconic-integration-tests --test binary_size_lto`)
- **Roadmap ID**: D05.02
- **Item**: LTO (or designed) flag documented; size delta smoke
- **Tests**: `crates/draconic-cli`, `tests/integration`
- **Targets**: native
