---
id: "l07-flags-cli-parse-argv-typed"
title: "L07 Flags/CLI parse: argv → typed options/positionals"
kind: task
status: completed
mode: afk
blocked-by: []
tags: []
created_at: "2026-09-04T13:27:18Z"
updated_at: "2026-09-05T00:45:00Z"
---

# L07 Flags/CLI parse: argv → typed options/positionals

## Blocked by

None.

## Done

ROADMAP L07 is implemented test-first on both targets: a Program can parse argv into typed options and leftover positionals through the designed flags surface; `stdlib/flags` fixtures are green and L07 is `done`.

## Context

Roadmap ID **L07** (`Flags/CLI parse: argv → typed options/positionals`). Stdlib location: honest portable libs a simple service needs. L07.01 and L07.02 land the per-class long/short/positional parse and typed-options/help fixtures; this sitting unifies them as one argv → typed options/positionals library a Program can use on both targets. Tests under `tests/conformance` fixtures `stdlib/flags`. Harness `tests/conformance/tests/stdlib_flags.rs`. Mark L07 `done` only when those tests are green. Not L07.01–L07.02 as separate atoms, H01 process args/env/exit, L05 test framework, L06 logging, Node `util.parseArgs` identity, or a full GNU getopt clone.

## Verify

`cargo test -p draconic-conformance --test stdlib_flags` prints `test result: ok.` Workspace `cargo test --workspace` stays green. L07 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (L07), `tests/conformance/fixtures/stdlib/flags`, `tests/conformance/tests/stdlib_flags.rs`, stdlib flags surface as needed for both targets

## Links

[[s-l07]] [[ticket-85-l07-flags-cli-parse-argv-typed]]
