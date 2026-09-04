---
id: "s-c04-workspace-timeout"
title: "C04 workspace tests finish"
kind: slice
status: met
sprint: "platform"
tags: []
created_at: "2026-09-04T13:39:23Z"
updated_at: "2026-09-04T13:49:31Z"
claimed-by: 0419ef55-e7c5-4442-9332-d2ced216cf9e
---

# C04 workspace tests finish

## Why

Review of [[s-c04]] left ROADMAP C04 unfinished: O1 (`test_cmd`), O2 (`cli_test_jobs`), and O3 (`cli_test_aggregate_order`) held, but O4 `cargo test --workspace` timed out at 120s. The concurrency location still needs the C04 Loop to leave the workspace green, not only the parallel `draconic test` harness.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP C04 work. The CLI `test_cmd` tests and the integration jobs and aggregate-order tests stay green. If the hang comes from the C04 change, fix that parallel-test worker-pool and aggregate-exit surface so those checks hold. Mark C04 `done` only when those tests are green.

## Blocked by

None.

## Non-goals

- **Re-opening [[s-c04]]**: that slice stays sealed `failed`
- **C04.01**: `draconic test` runs fixtures on worker pool (N>1) (already `done`)
- **C04.02**: Deterministic aggregate exit code + stable failure summary order (already `done`)
- **C01**: Worker / OS thread spawn isolate
- **C02**: Message-passing channels
- **C03**: `once` / thread-safe init
- **C05**: Structured cancellation / timeout helpers
- **C06**: Shared-memory atomics (later; not v1 bar)
- **L05**: In-language test framework
- Test262 runner parallelism (S02 / E19.02)

## Oracle checklist

- [x] O1: workspace tests finish after the C04 Loop
  CHECK: cargo test --workspace --offline --lib --bins && cargo test -p draconic-cli --test test_cmd --offline
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=cb1f97eada3e1f57 bytes=93989 at=2026-09-04T13:49:14.724Z

- [x] O2: C04 worker-pool and aggregate-exit behavior stay locked by the CLI test command tests
  CHECK: cargo test -p draconic-cli --test test_cmd
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=ed5252c48b95a139 bytes=3919 at=2026-09-04T13:49:16.277Z

- [x] O3: C04.01 multi-fixture worker pool (N>1) is locked by the integration jobs tests
  CHECK: cargo test -p draconic-integration-tests --test cli_test_jobs
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=f09f3e37214f9031 bytes=2965 at=2026-09-04T13:49:16.513Z

- [x] O4: C04.02 deterministic aggregate exit and stable FAIL order are locked by the integration aggregate-order tests
  CHECK: cargo test -p draconic-integration-tests --test cli_test_aggregate_order
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=0777e364a1bdd38e bytes=2946 at=2026-09-04T13:49:16.697Z

## Pool

Durable links to task-pool ids. Never drop them.

- `[[c04-workspace-timeout]]`

## See also

ROADMAP.md C04, `crates/draconic-cli/src/cmd_test.rs`, `crates/draconic-cli/tests/test_cmd.rs`, `tests/integration/tests/cli_test_jobs.rs`, `tests/integration/tests/cli_test_aggregate_order.rs`, CONTEXT.md, [[concurrency]], [[s-c04]], [[ticket-123-c04-workspace-timeout]].
