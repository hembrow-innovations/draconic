---
title: Compiler path returns Diagnostic
impact: CRITICAL
impactDescription: anyhow and string errors drop span and codes
tags: [err, diagnostic]
---

## Compiler path returns Diagnostic

Lexer through backends return `Result<T, Diagnostic>`. Build with `Diagnostic::new(message, span)`, then chain `.with_code` / `.with_help`. IO and dummy locations use `Span::dummy()`.

**Incorrect:**

```rust
fn compile_source(source: &str) -> anyhow::Result<Module> {
    parse(source).map_err(|e| anyhow::anyhow!("{e}"))?;
    todo!()
}
```

**Correct:**

```rust
pub fn compile_source(source: &str) -> Result<Module, Diagnostic> {
    let checked = check_source(source)?;
    Ok(lower(&checked))
}

let err = std::fs::read_to_string(entry).map_err(|e| {
    Diagnostic::new(format!("read {}: {e}", entry.display()), Span::dummy())
})?;
```

**Notes.** `thiserror` is pinned and unused in `.rs`. Diagnostics implements `std::error::Error` by hand. pkg is the other style — see `err-pkg-enum`. Tests may `.unwrap()` / `.expect("compile")`. Product frontend has zero unwraps. See `err-codes`.
