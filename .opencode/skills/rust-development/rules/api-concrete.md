---
title: Concrete fns, not domain traits
impact: HIGH
impactDescription: Trait soup hides the one implementation
tags: [api, traits]
---

## Concrete fns, not domain traits

The compiler is concrete `pub fn`s. Traits in this tree are `Display`, `Error`, and `From`. Do not add a `Compiler` / `Backend` / `Host` trait until two adapters exist.

**Incorrect:**

```rust
pub trait Backend {
    fn emit(&self, module: &Module) -> Result<String, Diagnostic>;
}
```

**Correct:**

```rust
pub fn emit_js(module: &Module) -> Result<String, Diagnostic>
pub fn emit_llvm_ir(module: &Module) -> Result<String, Diagnostic>
```

**Notes.** One adapter is a hypothetical seam; two adapters make it real (`arch-deep-modules`). LLVM already uses private `is_*_module` / `emit_*` fns, not a trait object. See `vis-pub-crate`.
