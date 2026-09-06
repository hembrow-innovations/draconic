---
title: CONTEXT.md vocabulary
impact: MEDIUM
impactDescription: Substituted words drift Dual worlds and the Loop
tags: [vocab, glossary]
---

## CONTEXT.md vocabulary

Use glossary terms: Program, Frontend, Linker, IR, JS backend, LLVM backend, Runtime, Embed, Dual worlds, JS value, Native type, Checker, Portable program, Native-only, JS-only, Catchable exception, Conformance suite, Roadmap, Loop.

**Incorrect:** calling the whole product "the compiler," IR "the AST," native types "primitives," or catchable exceptions "panics."

**Correct:** Compiler is the Rust program that emits artifacts. Toolchain is Compiler + Runtime + Embed + CLI. Frontend is parse/bind/check/lower. Panic/abort is R04.02, not `try`/`catch`.

**Notes.** Changing a term is **domain-modeling** plus **docs**, not a silent rename in a crate. Avoid list is in `CONTEXT.md`.
