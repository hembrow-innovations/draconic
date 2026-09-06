---
title: Lowering keeps per-call context
impact: MEDIUM
impactDescription: Process-global lower state races tests
tags: [own, lower, state]
---

## Lowering keeps per-call context

IR lowering uses a per-call `LowerCtx`. No process-global compiler state. Stages take `&` / `&mut self` on a struct.

**Incorrect:**

```rust
thread_local! {
    static LOCALS: RefCell<Vec<Local>> = RefCell::new(Vec::new());
}
pub fn lower(checked: &CheckedProgram) -> Module {
    LOCALS.with(|l| l.borrow_mut().clear());
    /* … */
}
```

**Correct:** a `LowerCtx` owned by `lower`, passed down the recursion.

**Notes.** Allowed thread-locals: linker active package context, LLVM JS interp `this` / `new.target`. See `own-thread-local` and `own-box-ast`.
