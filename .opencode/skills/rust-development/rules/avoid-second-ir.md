---
title: No second IR or AST
impact: LOW
impactDescription: WASM-as-IR and typed-AST forks split backends
tags: [avoid, ir]
---

## No second IR or AST

Do not add a per-backend typed AST, a WASM IR, or a second `Diagnostic` type. Both backends consume `draconic-ir::Module`.

**Incorrect:** `draconic-ir-js` and `draconic-ir-llvm` crates, or `CliError` that duplicates `Diagnostic` fields.

**Correct:** one IR, two emitters, one `Diagnostic`.

**Notes.** ADR-0002 rejected WASM-as-IR. See `ir-shared` and `err-diagnostic`.
