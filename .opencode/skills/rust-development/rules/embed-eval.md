---
title: Embed re-enters compile_source
impact: HIGH
impactDescription: A second eval parser forks Script policy
tags: [embed, eval]
---

## Embed re-enters compile_source

`eval_source` goes `compile_source` → small IR interpreter. Size / alloc / time budgets fail closed as diagnostics, not catchable JS.

**Incorrect:**

```rust
pub fn eval_source(source: &str) -> Result<EmbedValue, Diagnostic> {
    let ast = draconic_parser::parse(source)?;
    interpret_ast(&ast)
}
```

**Correct:**

```rust
pub fn eval_source(source: &str) -> Result<EmbedValue, Diagnostic> {
    let module = draconic_frontend::compile_source(source)?;
    interpret_ir(&module)
}
```

**Notes.** ADR-0004: full ECMA-262 including eval; Embed on native. Budgets are diagnostics. Fuzz hook: `fuzz_eval` (`test-fuzz-hooks`). See `crate-frontend-entry`.
