---
id: "architecture-parser"
title: "Frontend parser"
kind: architecture
description: "Script vs Module parse goals, AST construction, fail-fast diagnostics, and parser fuzz."
domain: draconic
area: frontend
tags: [architecture, frontend, parser]
created_at: "2026-09-06"
updated_at: "2026-09-06"
---

# Frontend parser

## Overview

`draconic-parser` is a recursive-descent parser from [[architecture-lexer|tokens]] to [[architecture-ast|Program]] (Roadmap B02, done). Public goals are Script (`parse`) and Module (`parse_module`). There is no error recovery: the first `Diagnostic` stops the parse. A workspace-excluded fuzz package (R05.01, done) treats Ok and Err as success and panics as failure.

## Context

[[CONTEXT]] distinguishes Program (toolchain input) from ECMAScript Module. Script vs Module is a parse goal, not a file extension. The [[architecture-frontend|Frontend]] facade chooses which function to call and when to invoke the [[architecture-linker|Linker]]. This crate must not read the filesystem or flatten import graphs.

## Design

### Crate layout

- [x] **src/lib.rs**: `parse`, `parse_module`, `parse_and_dump`, `parse_module_and_dump`, re-export `dump_ast`, private `Parser`, large unit tests
- [x] **src/fuzz.rs**: `fuzz_parse` (R05); re-exported at crate root
- [x] **fuzz/Cargo.toml**: standalone package `draconic-parser-fuzz`, `[workspace]` empty so it is excluded from `cargo test --workspace`
- [x] **fuzz/harness/parse_harness.rs**: designed stdin/file harness (R05.01)
- [x] **fuzz/fuzz_targets/parse.rs**: optional libFuzzer target behind feature `libfuzzer`

### Public API

- **parse(source)**: `Lexer::new` (Script HTML comments allowed) then `Parser::new(tokens, false)`. Starts `[~Await]`, not strict until a `"use strict"` directive.
- **parse_module(source)**: `Lexer::new_module` then `Parser::new(tokens, true)` with `in_strict = true` and `[+Await]` (top-level `await`). `yield` is reserved at top level. Annex B HTML-like comments are rejected by the lexer.
- **parse_and_dump / parse_module_and_dump**: parse then [[architecture-ast]] `dump_program`. Used by tests and `draconic parse` (B03 lives in CLI, not this crate).
- **fuzz_parse(data: &[u8])**: lossy UTF-8 decode, then `parse` and `parse_module`. Diagnostics discarded.

### Parser state (private)

`Parser` is not public. Flags: `allow_in` (for-header LHS), `in_generator`, `in_await_context`, `in_strict`, `is_module`, `using_container_depth` / `forbid_direct_using` (`using` / `await using`), nested class private-name stack, `new_target_depth`, `super_property_depth`, `prologue_had_legacy_escape`.

Identifier rules in this crate: reserved words always invalid as bindings; `yield` reserved in generators and strict; `await` reserved in modules and `[+Await]`; strict FutureReservedWord in strict mode.

ASI: `can_asi_before_current` is true on `}`, EOF, or `preceded_by_line_terminator`.

Directive prologue: consecutive string-literal expression statements; `"use strict"` calls `activate_strict_from_directive`, which errors if a prior prologue string used legacy octal escapes (E19.69).

After the body, `check_stmt_private_refs` with an empty private-name list rejects `#` refs outside any class (E19.39).

### What it produces

Every [[architecture-ast]] statement, expression, class, module, and type-annotation category the AST defines. Decorators (`@`, E19.78) are parsed and discarded: they do not appear on AST nodes. `import.meta` is Module-only. Cover grammar, assignment patterns, and ASI are implemented in `lib.rs`; this note does not restate ECMA-262 productions.

### Error recovery

None. `Result<Program, Diagnostic>` is first-error. Invalid syntax does not produce a partial tree. Fuzz success is “no panic”, not “always Ok”.

### Fuzz (R05 / R05.01)

`fuzz_parse` is the designed parser entry. Crate tests cover empty, valid script/module, invalid syntax, non-UTF-8 bytes, and nested garbage.

Standalone package (not the root workspace):

- **parse_harness**: read argv[1] or stdin, call `fuzz_parse`. Build with `cargo build --manifest-path crates/draconic-parser/fuzz/Cargo.toml --bin parse_harness`.
- **parse** (libFuzzer): `cargo fuzz run parse --fuzz-dir crates/draconic-parser/fuzz` with feature `libfuzzer`.

Embed/runtime fuzz is R05.02, not this crate.

### Tests

`lib.rs` `#[cfg(test)]` locks B02 dump snapshots (`parse_and_dump` exact strings), Script vs Module (`await`, `import.meta`, HTML comments), and ECMA early errors. `fuzz.rs` tests lock no-panic.

## Trade-offs

- **Fail fast vs IDE recovery**: one diagnostic keeps Conformance snapshots and fuzz harnesses simple. Multi-error parse is not in this crate.
- **Decorators syntax-only**: grammar accepted so Test262/E19.78 does not fail at parse; no AST/runtime meaning yet.
- **Fuzz out of workspace**: libFuzzer optional; designed harness stays buildable without cargo-fuzz.

## Consequences

Callers must not gate Module on a source substring. [[architecture-frontend]] detects ESM from parsed `Stmt` variants, then may call [[architecture-linker]]. Dump stability for snapshots is owned by [[architecture-ast]] `dump_program`; parser tests are the lock. Do not parse again after the Frontend has a Program.
