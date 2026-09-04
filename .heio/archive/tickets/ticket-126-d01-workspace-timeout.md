---
id: "ticket-126-d01-workspace-timeout"
title: "D01 workspace tests did not finish (O4 timeout)"
kind: ticket
status: closed
labels: feature
tags: []
sprint: platform
created_at: "2026-09-04T13:59:41Z"
updated_at: "2026-09-04T20:50:41.788Z"
slice: "s-d01-workspace-timeout"
caused-by: s-d01
failed: true
intent: fix
claimed-by: 790df251-5800-4e01-90e9-20ef96566e96
---

# D01 workspace tests did not finish (O4 timeout)

- **caused-by**: s-d01
- **failed oracle**: O4
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=timeout match=yes bytes=76464 at=2026-09-04T13:58:55.453Z
- **O1**: met (`cargo test -p draconic-integration-tests --test release_binaries`)
- **O2**: met (`cargo test -p draconic-integration-tests --test install_script`)
- **O3**: met (`cargo test -p draconic-integration-tests --test install_smoke`)
- **Roadmap ID**: D01
- **Item**: Release binaries + install script; one-line install to PATH
- **Tests**: `tests/integration` (`release_artifact`, `install_script`, `install_smoke`)
- **Targets**: compiler
