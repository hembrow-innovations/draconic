---
id: "l07-02-typed-options-bool-string-number"
title: "L07.02 Typed options (bool/string/number); help text as designed"
kind: task
status: ready
tags: []
created_at: "2026-09-02T22:27:40Z"
updated_at: "2026-09-02T22:27:40Z"
---

# L07.02 Typed options (bool/string/number); help text as designed

## Done

ROADMAP L07.02 is implemented test-first on both targets: a Program can parse typed options (bool/string/number) and produce help text as designed through the flags surface; `stdlib/flags` fixtures are green and L07.02 is `done`.

## Context

Roadmap ID **L07.02** (`Typed options (bool/string/number); help text as designed`). Stdlib location: honest portable libs a simple service needs. A Program can parse typed options (bool/string/number) and produce help text as designed through the flags surface. Tests under `tests/conformance` fixtures `stdlib/flags`. Harness `tests/conformance/tests/stdlib_flags.rs`. Mark L07.02 `done` only when those tests are green. Not L07 parent remainder, L07.01 long/short flags + positionals, H01 process argv, L05 test framework, L06 logging, Node `util.parseArgs` identity, or a full GNU getopt clone.

## Verify

`cargo test -p draconic-conformance --test stdlib_flags` prints `test result: ok.` Workspace `cargo test --workspace` stays green. L07.02 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (L07.02), `tests/conformance/fixtures/stdlib/flags`, `tests/conformance/tests/stdlib_flags.rs`, stdlib flags surface as needed for both targets

## Links

[[s-l07-02]] [[ticket-87-l07-02-typed-options-bool-string-number]]
