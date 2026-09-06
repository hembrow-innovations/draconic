---
title: types
section: reference
status: not-yet
---

# types

Types are TypeScript-inspired. The Checker is not tsc: it does not compile existing TypeScript projects, and it does not match tsc flag-for-flag. The JS backend emits JavaScript, not TypeScript. Drop-in TypeScript migration is out of scope so native types and Dual worlds are not constrained by tsc's type-erasure model.

Two universes in one Program:

- JS values are heap-managed and follow JavaScript semantics.
- A native type is a static unboxed systems type such as `i32`, `i64`, or a fixed struct, outside the GC heap. Do not call it a primitive.

Backend fit:

- A portable program is accepted by both backends with equivalent observable behavior after documented polyfills.
- Native-only features must hard-error on the JS backend, never silent wrong code.
- JS-only features must hard-error on the LLVM backend.

Dual-world boundaries are listed under [Dual-world rules](dual-world-rules.html). Learn path: [from JavaScript](from-javascript.html), then [Dual worlds](dual-worlds.html), then [native types](native-types.html).
