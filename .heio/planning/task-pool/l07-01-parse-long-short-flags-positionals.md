---
id: "l07-01-parse-long-short-flags-positionals"
title: "L07.01 Parse long/short flags + positionals from string array"
kind: task
status: ready
tags: []
created_at: "2026-09-02T13:48:03Z"
updated_at: "2026-09-02T13:48:03Z"
---

# L07.01 Parse long/short flags + positionals from string array

## Done

ROADMAP L07.01 is implemented test-first on both targets: a Program can parse long flags (`--name`), short flags (`-n`), and leftover positionals from a string array through the designed flags surface; `stdlib/flags` fixtures are green and L07.01 is `done`.

## Context

Roadmap ID **L07.01** (`Parse long/short flags + positionals from string array`). Stdlib location: honest portable libs a simple service needs. A Program can parse long flags, short flags, and leftover positionals from a string array through the designed flags surface. Tests under `tests/conformance` fixtures `stdlib/flags`. Harness `tests/conformance/tests/stdlib_flags.rs`. Mark L07.01 `done` only when those tests are green. Not L07 parent remainder, L07.02 typed options/help text, H01 process argv, L05 test framework, L06 logging, Node `util.parseArgs` identity, or a full GNU getopt clone.

## Verify

`cargo test -p draconic-conformance --test stdlib_flags` prints `test result: ok.` Workspace `cargo test --workspace` stays green. L07.01 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (L07.01), `tests/conformance/fixtures/stdlib/flags`, `tests/conformance/tests/stdlib_flags.rs`, stdlib flags surface as needed for both targets

## Links

[[s-l07-01]] [[ticket-86-l07-01-parse-long-short-flags-positionals]]
