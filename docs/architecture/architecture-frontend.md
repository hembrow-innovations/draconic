---
id: "architecture-frontend"
title: "Frontend facade"
kind: architecture
description: "draconic-frontend assembles parse, check, and lower; Script vs Module policy lives here."
domain: draconic
area: frontend
tags: [architecture, frontend, facade]
created_at: "2026-09-06"
updated_at: "2026-09-06"
---

# Frontend facade

## Overview

`draconic-frontend` is the only crate callers should use to turn Draconic source (or an entry path) into shared [[architecture-ir|IR]]. It owns Script vs Module (link) policy, then check → lower. Stage crates ([[architecture-lexer]], [[architecture-parser]], [[architecture-ast]], [[architecture-check]], [[architecture-diagnostics]]) are implementation details of this path. [[CONTEXT]] names this assembly the Frontend.

## Context

Without a facade, CLI, Embed, and tests each wired parser, checker, and IR and guessed Module from source text. That drifted (top-level `await`, HTML comments, ESM). The Linker flattens a static import graph to one Program; it is not part of the Parser product. The rest of [[architecture-pipeline]] starts after `lower`.

## Design

### Crate layout

- [x] **src/lib.rs**: `compile_source`, `compile_source_module`, `compile_path`, `check_source`, `check_source_module`, `check_path`, Script/Module detection, unit tests
- [x] **Cargo.toml**: `draconic-ast`, `draconic-diagnostics`, `draconic-parser`, `draconic-check`, `draconic-ir`, `draconic-linker`

No other `src/*.rs` files. Re-exports: `CheckedProgram`, `draconic_ir::Module`.

### Public compile API

- **compile_source**: parse+check as Script, then `lower`. No filesystem. Relative imports are not resolved. Top-level `await` is rejected (Script goal).
- **compile_source_module**: `parse_module` + `check_module` + `lower` (E19.28). Top-level `await` allowed. Still no link graph.
- **compile_path**: read entry, choose Script parse or Module link (parse-driven), check with matching goal, lower.

`check_source` / `check_source_module` / `check_path` stop before lower (for tests and tools that want `CheckedProgram`).

### Script vs Module on a path

`load_program`:

1. Read the file (`Diagnostic` with dummy span on IO failure).
2. Try `parse` (Script). If Ok and the Program body contains ESM import/export statements, call `link_entry` and mark module goal.
3. If Script parse Ok without module syntax, return that Program as Script.
4. If Script parse fails, try `parse_module`. If that Program has module syntax, `link_entry` and module goal. Otherwise return the original Script diagnostic.

Module syntax is AST-level, not a substring heuristic:

- `Stmt::ImportDeclaration`
- `Stmt::ExportNamedDeclaration`
- `Stmt::ExportDefaultDeclaration`
- `Stmt::ExportAllDeclaration`

So `let import_name = 1` is Script; `export let x = 1` is Module. Export plus top-level `await` can fail Script parse because `await` is IdentifierReference under `[~Await]` (E19.52); the Module retry covers that.

Linked entries use Module check (top-level `await` allowed). `link_entry` is [[architecture-linker]].

### What this crate does not do

- Does not call `check_for_target`. Host API js/native availability is [[architecture-check]] `check_for_target`, not the facade.
- Does not emit JS or LLVM. That is after [[architecture-ir]].
- Does not tokenize or dump AST; CLI `draconic parse` uses the parser dump helpers.

### Tests

Inline tests: Script compile, path Script skips link, path Module links a relative import, module-syntax detection ignores the word `import` in an identifier, `compile_source_module` allows top-level `await` while Script diagnostics, path export+await uses Module.

## Trade-offs

- **Script-first then Module retry**: preserves Script semantics for files without ESM, including Annex B HTML comments, while still compiling `export let x = await 2`.
- **Parse-driven link**: extra parse before `link_entry` on module files; avoids false positives from comments and identifiers.
- **No target in the facade**: portable IR first; native-only host/`extern` hard-errors are a later check API.

## Consequences

CLI, Embed, and Conformance should call these functions rather than `parse` + `check` + `lower` ad hoc. New parse goals belong here, not in backends. Sibling notes: [[architecture-diagnostics]], [[architecture-lexer]], [[architecture-parser]], [[architecture-ast]], [[architecture-check]]. Downstream: [[architecture-ir]], [[architecture-linker]], [[architecture-pipeline]].
