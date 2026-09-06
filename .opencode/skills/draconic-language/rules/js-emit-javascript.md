---
title: JS backend emits JavaScript
impact: HIGH
impactDescription: TypeScript emit implies tsc compatibility
tags: [js, emit]
---

## JS backend emits JavaScript

The JS backend emits ECMAScript. It does not emit TypeScript. Types erase.

**Incorrect:** `.ts` output, `strictNullChecks` flags, or keeping `i32` syntax in the emitted file.

**Correct:** `emit_js(&module) -> EmittedJs`. Native scalars that N04 allows become JS numbers. Pointers hard-error before emit.

**Notes.** ADR-0005. See `prod-not-tsc` and `js-n04-polyfill`.
