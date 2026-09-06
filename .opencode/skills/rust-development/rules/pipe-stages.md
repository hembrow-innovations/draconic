---
title: Stages stay in their crates
impact: CRITICAL
impactDescription: Cross-crate lowering forks IR and policy
tags: [pipe, crate, stages]
---

## Stages stay in their crates

source → lexer → parser → AST → (linker if ESM) → check → IR → JS backend or LLVM backend. Runtime links into native binaries. Embed re-enters frontend at runtime.

**Incorrect:** lowering AST in the JS backend, or typechecking inside the parser, or detecting modules by scanning source text in the CLI.

**Correct:**

- Parse in `draconic-parser` (`parse` / `parse_module`)
- Link in `draconic-linker` (`link_entry`)
- Check in `draconic-check` (`check` / `check_for_target`)
- Lower in `draconic-ir` (`lower` — infallible after check)
- Emit in `draconic-backend-js` / `draconic-backend-llvm`
- Orchestrate in `draconic-frontend`

**Notes.** Product callers use frontend (`crate-frontend-entry`). `Diagnostic` and `Span` travel with every stage. Do not retarget a foreign IR. See `ir-shared` and `pipe-script-module`.
