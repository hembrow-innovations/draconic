---
title: LLVM backend emits IR text then clang
impact: HIGH
impactDescription: llvm-sys binds a toolchain this repo does not ship
tags: [backend, llvm]
---

## LLVM backend emits IR text then clang

`emit_llvm_ir(&Module) -> Result<String, Diagnostic>` writes LLVM IR text. Dispatch a chain of `is_*_module` / `emit_*`. Then `clang` plus Runtime C (`draconic_rt.c`). No inkwell, no llvm-sys.

**Incorrect:**

```rust
use inkwell::context::Context;
pub fn emit(module: &Module) -> LLVMModule { /* … */ }
```

**Correct:**

```rust
pub fn emit_llvm_ir(module: &Module) -> Result<String, Diagnostic> { /* text */ }
```

Then invoke clang against that text and the runtime C sources.

**Notes.** Empty program may emit B08 hello. Non-empty unsupported IR must hard-error (`err-hard-error`). Host adapters live in `host_fs.rs`, `host_tcp.rs`, … See `size-file-budget`.
