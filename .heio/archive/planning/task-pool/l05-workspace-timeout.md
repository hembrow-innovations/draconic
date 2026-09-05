---
id: "l05-workspace-timeout"
title: "L05 workspace tests finish"
kind: task
status: completed
mode: afk
blocked-by: []
tags: []
created_at: "2026-09-04T20:50:50Z"
updated_at: "2026-09-04T21:00:43Z"
---

# L05 workspace tests finish

## Blocked by

None.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP L05 work; the stdlib testing conformance tests and the CLI `draconic test` command tests stay green.

## Context

Roadmap ID **L05** (`In-language test framework (`describe`/`it`/`expect` or designed) via `draconic test``). Review of [[s-l05]] left O3 unmet: `cargo test --workspace` timed out at 120s while O1 (`stdlib_testing`) and O2 (`test_cmd`) stayed green. The stdlib location still needs the L05 Loop to leave the workspace green, not only the in-language `describe`/`it`/`expect` fixtures and `draconic test` CLI aggregate-exit tests. If the hang comes from the L05 change, fix that in-language test-framework surface so the workspace check and those fixtures hold. Mark L05 `done` only when those tests are green. Not L05.01–L05.04 (already `done`), C04 parallel worker pool, L06 logging, or the Test262 runner (S02 / E19.02). Do not re-open [[s-l05]].

## Verify

`cargo test --workspace --offline --lib --bins && cargo test -p draconic-conformance --test stdlib_testing --offline && cargo test -p draconic-cli --test test_cmd --offline` prints `test result: ok.` and finishes (does not hang). `cargo test -p draconic-conformance --test stdlib_testing` still prints `test result: ok.` `cargo test -p draconic-cli --test test_cmd` still prints `test result: ok.` L05 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (L05), `tests/conformance/tests/stdlib_testing.rs`, `tests/conformance/fixtures/stdlib/testing`, `crates/draconic-cli/tests/test_cmd.rs`, `crates/draconic-backend-llvm/src/es_testing.rs`, in-language testing surface as needed to unhang workspace tests after L05

## Links

[[s-l05-workspace-timeout]] [[ticket-183-l05-workspace-timeout]] [[s-l05]]
