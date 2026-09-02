---
id: "c04-parallel-draconic-test"
title: "C04 parallel draconic test surface"
kind: task
status: completed
tags: []
created_at: "2026-09-02T13:09:59Z"
updated_at: "2026-09-02T14:40:00Z"
---

# C04 parallel draconic test surface

## Done

ROADMAP C04 is implemented test-first on the compiler: `draconic test` runs fixtures on a worker pool (N>1, including default and `--jobs`), any fixture failure yields aggregate exit 1 even with passing siblings, and FAIL summary order is stable by fixture id; CLI and integration tests are green and C04 is `done`.

## Context

Roadmap ID **C04** (Parallel `draconic test`: multi-fixture workers; deterministic aggregate exit). C04.01–C04.02 already land the per-class worker-pool and aggregate-exit tests; this sitting unifies them as one honest parallel-test surface on the compiler. Tests under `crates/draconic-cli` and `tests/integration`. Mark C04 `done` only when those tests are green. Not C01–C03, C05, C06, L05, or Test262 runner parallelism (S02 / E19.02).

## Verify

`cargo test -p draconic-cli --test test_cmd` prints `test result: ok.` `cargo test -p draconic-integration-tests --test cli_test_jobs` prints `test result: ok.` `cargo test -p draconic-integration-tests --test cli_test_aggregate_order` prints `test result: ok.` Workspace `cargo test --workspace` stays green. C04 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (C04), `crates/draconic-cli/src/cmd_test.rs`, `crates/draconic-cli/tests/test_cmd.rs`, `tests/integration/tests/cli_test_jobs.rs`, `tests/integration/tests/cli_test_aggregate_order.rs`, compiler test-runner paths as needed for the parent surface

## Links

[[s-c04]] [[ticket-74-c04-parallel-draconic-test-multi-fixture]]
