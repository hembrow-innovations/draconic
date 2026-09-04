---
id: "l05-in-language-test-framework-describe"
title: "L05 In-language test framework (`describe`/`it`/`expect` or designed) via `draconic test`"
kind: task
status: completed
mode: afk
blocked-by: []
tags: []
created_at: "2026-09-04T13:25:51Z"
updated_at: "2026-09-04T20:41:55Z"
---

# L05 In-language test framework (`describe`/`it`/`expect` or designed) via `draconic test`

## Blocked by

None.

## Done

ROADMAP L05 is implemented test-first on both targets: a Program can register tests with `describe`/`it` (or designed names), assert with `expect` matchers, nest describes with hooks as designed, and `draconic test` runs that suite and aggregates its exit with fixture results; `stdlib/testing` and CLI test-command tests are green and L05 is `done`.

## Context

Roadmap ID **L05** (`In-language test framework (`describe`/`it`/`expect` or designed) via `draconic test``). Stdlib location: honest portable libs a simple service needs. L05.01–L05.04 already land `describe`/`it` registration via `draconic test`, `expect` matchers with failure messages, nested describe plus before/after hooks, and CLI aggregate exit with the fixture runner; this sitting unifies them as one testing library a Program can run with `draconic test` on both targets. Tests under `tests/conformance` fixtures `stdlib/testing` and `crates/draconic-cli`. Harnesses `tests/conformance/tests/stdlib_testing.rs` and `crates/draconic-cli/tests/test_cmd.rs`. Mark L05 `done` only when those tests are green. Not L05.01–L05.04 as separate atoms, C04 parallel worker pool, L06 logging, or the Test262 runner (S02 / E19.02).

## Verify

`cargo test -p draconic-conformance --test stdlib_testing` prints `test result: ok.` `cargo test -p draconic-cli --test test_cmd` prints `test result: ok.` Workspace `cargo test --workspace` stays green. L05 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (L05), `tests/conformance/fixtures/stdlib/testing`, `tests/conformance/tests/stdlib_testing.rs`, `crates/draconic-cli/tests/test_cmd.rs`, `crates/draconic-backend-llvm/src/es_testing.rs`, in-language testing surface as needed for both targets

## Links

[[s-l05]] [[ticket-83-l05-in-language-test-framework-describe]]
