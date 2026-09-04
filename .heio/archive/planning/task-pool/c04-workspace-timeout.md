---
id: "c04-workspace-timeout"
title: "C04 workspace tests finish"
kind: task
status: completed
mode: afk
blocked-by: []
tags: []
created_at: "2026-09-04T13:41:17Z"
updated_at: "2026-09-04T13:46:57Z"
---

# C04 workspace tests finish

## Blocked by

None.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP C04 work; the CLI `test_cmd` tests and the integration jobs and aggregate-order tests stay green.

## Context

Roadmap ID **C04** (Parallel `draconic test`: multi-fixture workers; deterministic aggregate exit). Review of [[s-c04]] left O4 unmet: `cargo test --workspace` timed out at 120s while `test_cmd`, `cli_test_jobs`, and `cli_test_aggregate_order` stayed green. If the hang comes from the C04 change, fix that parallel-test worker-pool and aggregate-exit surface so those checks hold. Mark C04 `done` only when those tests are green. Not C04.01–C04.02 as separate atoms, C01–C03, C05–C06, L05, or Test262 runner parallelism. Do not re-open [[s-c04]].

## Verify

`cargo test --workspace --offline --lib --bins && cargo test -p draconic-cli --test test_cmd --offline` prints `test result: ok.` and finishes (does not hang). `cargo test -p draconic-cli --test test_cmd`, `cargo test -p draconic-integration-tests --test cli_test_jobs`, and `cargo test -p draconic-integration-tests --test cli_test_aggregate_order` still print `test result: ok.` C04 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (C04), `crates/draconic-cli/src/cmd_test.rs`, `crates/draconic-cli/tests/test_cmd.rs`, `tests/integration/tests/cli_test_jobs.rs`, `tests/integration/tests/cli_test_aggregate_order.rs`, parallel-test worker-pool and aggregate-exit paths as needed to unhang workspace tests after C04

## Links

[[s-c04-workspace-timeout]] [[ticket-123-c04-workspace-timeout]] [[s-c04]]
