---
id: "task-770-parser-context"
title: "Parser one context value"
kind: task
status: completed
mode: afk
blocked_by: []
sprint: "rust-dev-audit"
slice: "slice-706-parser-file-budget"
tags: []
created_at: "2026-09-09T17:30:00Z"
updated_at: "2026-09-09T20:50:00Z"
---
# Parser one context value

## Blocked by

None. Prefactor for [[task-718-parser-extract-stmt]].

## Done

Parser grammar parameters (allow_in, in_generator, in_await_context, in_strict, is_module, using, private stack, new.target, super) push/pop as one context value. parse / parse_module unchanged. cargo test -p draconic-parser is green. lib.rs is not larger than 10483.

## Context

Dragons audit: Parser is already a deep module. Flag save/restore is the spaghetti, not missing files. Do this before extracting stmt/expr files so those files share one context, not eleven fields. Do not add a public parse context type.

## Verify

cargo test -p draconic-parser --offline. rg "let prev_" count in parser src is lower than 31. parse and parse_module still the public entries.

scope: crates/draconic-parser/src/lib.rs (and a private context sibling only if lib.rs would otherwise grow)

## Links

[[slice-706-parser-file-budget]]

## Agent Brief

**Category:** layout
**Summary:** Replace Parser flag spaghetti with one pushed context.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Intent (required when product behaviour changes):**
- Promise ids: none unless this task names a Diagnostic change
- Purpose: parse grammar parameters without save/restore blocks
- Contract-first: do not invent language behaviour

**Current behavior:**
Eleven Parser fields and dozens of prev_ restores.

**Desired behavior:**
One context push/pop. Same parse diagnostics and trees.

**Key interfaces:**
- parse / parse_module stay public
- Parser stays crate-private
- size-file-budget: lib.rs must not grow; a private context.rs ≤1000 is allowed

**Acceptance criteria:**
- [x] one context value owns the grammar flags
- [x] cargo test -p draconic-parser
- [x] parse / parse_module signatures unchanged
- [x] lib.rs line count ≤ 10483

## Gauntlet

- **round**: 1
- **command**: cargo test -p draconic-parser --offline; rg "let prev_" in parser src; lib.rs line count
- **win/lose**: win
- **gap**: none

**Out of scope:**
- Extracting parse_stmt.rs (task-718)
- Grammar changes
- New crates
- Language behaviour or ROADMAP rows

**Explain this part:**
Do not create a public ParserContext interface. Sibling files later must impl the same Parser, not a new module API.
