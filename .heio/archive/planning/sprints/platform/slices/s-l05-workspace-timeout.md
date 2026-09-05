---
id: "s-l05-workspace-timeout"
title: "L05 workspace tests finish"
kind: slice
status: met
sprint: "platform"
tags: []
created_at: "2026-09-04T20:48:30Z"
updated_at: "2026-09-04T21:02:57Z"
claimed-by: 84bef7c5-97e5-4514-b92d-2cd20cb2a449
---

# L05 workspace tests finish

## Why

Review of [[s-l05]] left ROADMAP L05 unfinished: O1 (`stdlib_testing`) and O2 (`test_cmd`) held, but O3 `cargo test --workspace` timed out at 120s. The stdlib location still needs the L05 Loop to leave the workspace green, not only the in-language `describe`/`it`/`expect` fixtures and `draconic test` CLI aggregate-exit tests.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP L05 work. The stdlib testing conformance tests and the CLI `draconic test` command tests stay green. If the hang comes from the L05 change, fix that in-language test-framework surface so the workspace check and those fixtures hold. Mark L05 `done` only when those tests are green.

## Blocked by

None.

## Non-goals

- **Re-opening [[s-l05]]**: that slice stays sealed `failed`
- **L05.01**: `describe` / `it` (or designed) register tests; run via `draconic test` (already `done`)
- **L05.02**: `expect` matchers: equality, truthiness; failure messages (already `done`)
- **L05.03**: Nested describe; before/after hooks as designed (already `done`)
- **L05.04**: CLI aggregates in-language suite exit codes with fixture runner (already `done`)
- **C04**: Parallel `draconic test` worker pool
- **L06**: Logging
- Test262 runner (S02 / E19.02)

## Oracle checklist

- [x] O1: workspace tests finish after the L05 Loop
  CHECK: cargo test --workspace --offline --lib --bins && cargo test -p draconic-conformance --test stdlib_testing --offline && cargo test -p draconic-cli --test test_cmd --offline
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=605229ffe0ee32e9 bytes=98884 at=2026-09-04T21:02:33.177Z

- [x] O2: L05 describe/it/expect and nested-hooks fixtures stay locked by the stdlib testing conformance tests
  CHECK: cargo test -p draconic-conformance --test stdlib_testing
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=f38b68cf678990c0 bytes=3332 at=2026-09-04T21:02:34.334Z

- [x] O3: L05 `draconic test` run and CLI aggregate-exit behavior stay locked by the CLI test command tests
  CHECK: cargo test -p draconic-cli --test test_cmd
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=a3ca90316a8f4a08 bytes=3982 at=2026-09-04T21:02:35.363Z

## Pool

Durable links to task-pool ids. Never drop them.

- `[[l05-workspace-timeout]]`

## See also

ROADMAP.md L05, `tests/conformance/tests/stdlib_testing.rs`, `tests/conformance/fixtures/stdlib/testing`, `crates/draconic-cli/tests/test_cmd.rs`, `crates/draconic-backend-llvm/src/es_testing.rs`, CONTEXT.md, [[stdlib]], [[s-l05]], [[ticket-183-l05-workspace-timeout]].
