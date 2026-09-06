---
title: Dual worlds stay explicit
impact: CRITICAL
impactDescription: Boxing native types into GC collapses N04
tags: [dual, native, gc]
---

## Dual worlds stay explicit

JS values are heap-managed (GC on native). Native types (`i32`, `i64`, layout structs, `Type::Ptr`) are unboxed and outside GC. Boundaries live at the type and lowering level.

**Incorrect:** representing `i32` as a GC JS number on the native path, or treating `*T` as an ordinary JS value.

**Correct:**

- Checker: `Type::Native(NativeType)` and `Type::Ptr` vs JS value types
- JS backend: native scalars / layouts polyfill as ordinary JS; pointers hard-error
- LLVM / runtime: unboxed native; JS values through the GC

**Notes.** Vocabulary is **Native type** and **JS value** (`CONTEXT.md`). ADR-0003. Pointers and `extern "C"` are native-only. See `dual-js-policy`.
