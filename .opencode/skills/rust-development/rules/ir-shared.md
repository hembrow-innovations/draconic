---
title: One shared IR, two backends
impact: CRITICAL
impactDescription: Per-backend ASTs fork semantics
tags: [ir, backend, adr]
---

## One shared IR, two backends

Both backends lower from `draconic-ir::Module` after Frontend. There is no typed-AST fork and no WASM-as-IR.

**Incorrect:**

```rust
pub fn emit_js_from_ast(program: &Program) -> String { /* … */ }
pub fn emit_llvm_from_ast(program: &Program) -> String { /* … */ }
```

**Correct:**

```rust
pub fn lower(checked: &CheckedProgram) -> Module { /* … */ }
pub fn emit_js(module: &Module) -> Result<String, Diagnostic> { /* … */ }
pub fn emit_llvm_ir(module: &Module) -> Result<String, Diagnostic> { /* … */ }
```

**Notes.** `lower` is infallible after check; it may `panic!` on "check already rejected this" invariants. ADR-0002. See `backend-js-emit` and `backend-llvm-text`.
