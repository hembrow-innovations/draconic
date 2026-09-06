---
title: Helpers are pub(crate)
impact: HIGH
impactDescription: Public internals freeze accidental seams
tags: [vis, api]
---

## Helpers are pub(crate)

Public crate API is `pub fn` / `pub struct` at the root (or `pub use`). Intra-crate helpers are `pub(crate)`. Nested modules stay private unless re-exported.

**Incorrect:**

```rust
pub fn is_es_expr_module(module: &Module) -> bool { /* … */ }
```

**Correct:**

```rust
pub(crate) fn is_es_expr_module(module: &Module) -> bool { /* … */ }
pub(crate) fn emit_es_expr(module: &Module) -> Result<String, Diagnostic> { /* … */ }
```

**Notes.** LLVM adapters are the pattern. AST / IR fields are mostly `pub` because every stage matches on them. Checker hides resolution maps and exposes methods (`BoundProgram::resolve`). See `api-concrete`.
