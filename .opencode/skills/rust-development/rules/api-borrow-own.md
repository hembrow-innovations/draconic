---
title: Borrow in, own out
impact: HIGH
impactDescription: Forced clones at the seam spread cost
tags: [api, ownership]
---

## Borrow in, own out

Public compiler fns take borrowed input (`&str`, `&Path`, `&CheckedProgram`, `&Module`) and return owned results (`Module`, `String`, `Diagnostic`). Constructors that store strings take `impl Into<String>`.

**Incorrect:**

```rust
pub fn compile_source(source: String) -> Result<Module, Diagnostic> { /* … */ }
pub fn emit_js(module: Module) -> Result<String, Diagnostic> { /* … */ }
```

**Correct:**

```rust
pub fn compile_source(source: &str) -> Result<Module, Diagnostic>
pub fn compile_path(entry: &Path) -> Result<Module, Diagnostic>
pub fn lower(checked: &CheckedProgram) -> Module
pub fn emit_js(module: &Module) -> Result<String, Diagnostic>
```

**Notes.** `Diagnostic::new(message: impl Into<String>, span)` and pkg `LockEntry::new(path: impl Into<String>, …)` are the constructor shape. See `api-concrete`.
