---
id: "d03-01-workspace-timeout"
title: "D03.01 workspace tests finish"
kind: task
status: completed
mode: afk
blocked-by: []
tags: []
created_at: "2026-09-04T14:14:27Z"
updated_at: "2026-09-04T14:30:39Z"
---

# D03.01 workspace tests finish

## Blocked by

None.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP D03.01 work; the `reproducibility_expectations` harness stays green.

## Context

Roadmap ID **D03.01** (Document reproducibility expectations for timestamps and paths). Review of [[s-d03-01]] left O2 unmet: `cargo test --workspace` timed out at 120s while O1 (`reproducibility_expectations`) stayed green. If the hang comes from the D03.01 change, fix that timestamp-and-path documentation surface so both the workspace check and those integration tests hold. Mark D03.01 `done` only when those tests are green. Not D03.02 emit identity, D03 parent remainder, D02 toolchain pin, D01 release binaries + install script, D04 cross-compile matrix, or D05 strip/LTO. Do not re-open [[s-d03-01]].

## Verify

`cargo test --workspace --offline --lib --bins && cargo test -p draconic-integration-tests --test reproducibility_expectations --offline` prints `test result: ok.` and finishes (does not hang). `cargo test -p draconic-integration-tests --test reproducibility_expectations` still prints `test result: ok.` D03.01 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (D03.01), `tests/integration/tests/reproducibility_expectations.rs`, `website/install.md`, `docs/`, timestamp-and-path documentation surface as needed to unhang workspace tests after D03.01

## Links

[[s-d03-01-workspace-timeout]] [[ticket-128-d03-01-workspace-timeout]] [[s-d03-01]]
