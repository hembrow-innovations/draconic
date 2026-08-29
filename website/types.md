---
title: types
section: reference
status: not-yet
---

# types

Types are TypeScript-inspired. The Checker is not tsc and does not compile existing TypeScript projects.

JS values are heap-managed and follow JavaScript semantics. A native type is a static unboxed systems type such as i32 or i64, outside the GC heap.

The JS backend emits JavaScript, not TypeScript. Native-only features must hard-error on the JS backend. JS-only features must hard-error on the LLVM backend.

Dual-world boundaries are listed under [Dual-world rules](dual-world-rules.html).
