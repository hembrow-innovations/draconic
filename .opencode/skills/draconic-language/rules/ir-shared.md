---
title: One IR both backends
impact: HIGH
impactDescription: Per-backend typed ASTs let semantics drift
tags: [ir, backend]
---

## One IR both backends

After Frontend, every Program lowers to one shared IR. The JS backend and LLVM backend both consume `draconic_ir::Module`.

**Incorrect:** lowering from a typed AST per backend, or making WASM bytecode the IR.

**Correct:** `draconic_ir::lower(&checked) -> Module`, then `emit_js(&module)` or `emit_llvm_ir(&module)`. wasm32-wasi is an extra LLVM object target, not a second IR.

**Notes.** ADR-0002. Backends may assume every `Expr` carries `ty`, imports are gone, and `as` is erased. LLVM may not assume a full ES program lowers; unclassified IR hard-errors. See `ir-after-link`.
