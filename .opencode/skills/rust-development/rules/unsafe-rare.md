---
title: Unsafe stays off the IR path
impact: MEDIUM
impactDescription: Unsafe in lowering hides alias bugs
tags: [unsafe, ir]
---

## Unsafe stays off the IR path

Unsafe is essentially unused in the compiler. CLI may wrap `isatty`. Do not add `unsafe` for IR, AST, or check.

**Incorrect:**

```rust
pub fn lower(checked: &CheckedProgram) -> Module {
    unsafe { transmute_locals(checked) }
}
```

**Correct:** safe lowering with `LowerCtx`. FFI lives in Runtime C (`draconic_rt_*`), not in `unsafe` Rust IR.

**Notes.** Need FFI from Rust: put it in runtime C / headers, or isolate a tiny documented block next to existing CLI `isatty`. See `backend-llvm-text`.
