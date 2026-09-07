---
id: "task-662-b03-parse-cli-spawn"
title: "Spawn draconic parse from the CLI Tests column"
kind: task
status: completed
mode: afk
blocked_by: []
sprint: "platform"
slice: "slice-661-b03-parse-cli-spawn"
tags: []
created_at: "2026-09-07T06:25:24Z"
updated_at: "2026-09-07T20:15:00Z"
---

# Spawn draconic parse from the CLI Tests column

## Blocked by

None.

## Done

`cargo test -p draconic-cli --test parse` prints `test result: ok.` The spawn would fail if `cmd_parse` were removed. `toolchain.cli:parse-ast` lists that spawn test. B03 stays `done`.

## Context

Roadmap B03 already ships `draconic parse <file>` as an AST dump. The CLI crate’s in-process `parse_sample_program` calls `parse_and_dump` and never execs the binary, so the Tests column named on B03 does not lock `cmd_parse`. Sibling verbs (`check`, `run`, `fmt`) already spawn `CARGO_BIN_EXE_draconic`. Install-smoke covers a shipped PATH binary outside this column. Add a CLI-crate integration test binary named `parse` that writes a tiny valid Program, runs `draconic parse <file>`, asserts exit 0 and a dump starting with `Program`. Point `toolchain.cli:parse-ast` at that test. Do not reopen B03. Do not change parse behaviour unless the spawn proves `cmd_parse` is broken.

## Verify

`cargo test -p draconic-cli --test parse` prints `test result: ok.` Slice O1 EVIDENCE is filled. B03 remains `done` on ROADMAP.md.

scope: `crates/draconic-cli` (integration test binary `parse` plus `cmd_parse` only if the spawn fails), `docs/specs/draconic/toolchain/` (contract test pointer and tests map), [[slice-661-b03-parse-cli-spawn]] EVIDENCE

## Links

[[slice-661-b03-parse-cli-spawn]] [[ticket-588-b03-parse-cli-untested]]

## Agent Brief

**Category:** bug
**Summary:** Lock B03’s Tests column by spawning the `draconic` binary with `parse` so `cmd_parse` cannot be deleted while tests stay green.

**Drain:** `/afk-task`. Unblocked. Claim `status: ready` then `mode: afk`.

**Skills:** load **tdd**, **rust-development**, **draconic-language**, **docs**, **spec**.

**Intent:**
- Promise ids: `toolchain.cli:parse-ast`
- Purpose: [[Toolchain purpose]]
- Contract-first: assert the existing parse-ast promise, add a test pointer for the CLI spawn, then write the failing spawn test, then code only if `cmd_parse` is broken
- Product behaviour of `draconic parse` does not change in this sitting

**Current behavior:**
`draconic parse` already dumps a Program AST. The CLI crate test named for B03 calls `parse_and_dump` in-process. No CLI-crate integration test execs the binary with `parse`. Deleting `cmd_parse` would leave that in-process test green. Install-smoke and release-binary PATH checks spawn parse outside the Roadmap Tests column.

**Desired behavior:**
An integration test binary named `parse` in the CLI crate spawns the `draconic` binary the same way `check` and `run` already do. It passes a valid Program file, expects exit 0, and asserts stdout is an AST dump starting with `Program`. `toolchain.cli:parse-ast` lists that spawn test alongside the existing dump and install-smoke pointers. B03 stays `done`.

**Key interfaces:**
- `cmd_parse`. CLI verb: read a Program file, print `parse_and_dump`, exit 0 on success. Missing path exits 2 with usage. Read or parse failure exits 1. Do not change this unless the spawn test proves it is broken.
- `parse_and_dump`. Parser helper that returns the dump string. Keep as the dump engine; the new test must still go through the CLI binary.
- `toolchain.cli:parse-ast`. Existing promise: `draconic parse` accepts a Program file and prints an AST dump for a valid Program.

**Acceptance criteria:**
- [x] `cargo test -p draconic-cli --test parse` prints `test result: ok.`
- [x] The test execs the CLI binary with the `parse` verb and a Program file (not in-process `parse_and_dump` alone)
- [x] Success path: exit 0 and dump starts with `Program`
- [x] `toolchain.cli:parse-ast` has a `test:` pointer for that spawn test
- [x] Toolchain tests map mentions the spawn test
- [x] B03 remains `done`
- [x] Slice O1 EVIDENCE is filled

**Out of scope:**
- Reopening B03 or marking it `todo`
- New parse flags, help titles, or a full error-path suite
- Rewriting install-smoke or release-binary PATH coverage
- Language semantics, Checker, backends
- Public site or Learn/Reference content
- Respect purpose Out of scope fences on [[Toolchain purpose]]

## Gauntlet

- **round 1**: `cargo test -p draconic-cli --test parse` — win. `test result: ok.` 1 passed. Diff locks `toolchain.cli:parse-ast` by spawning `CARGO_BIN_EXE_draconic parse`; no `cmd_parse` change; B03 stays done; purpose out-of-scope fences untouched.
