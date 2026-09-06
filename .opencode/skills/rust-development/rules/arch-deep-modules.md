---
title: Deep modules at real seams
impact: CRITICAL
impactDescription: Shallow pass-throughs spread change and block tests
tags: [arch, module, seam, depth]
---

## Deep modules at real seams

A **module** is anything with an interface and an implementation. Design **deep** modules: lots of behaviour behind a small interface, placed at a clean **seam**. Callers and tests cross that same seam.

Use these words: module, interface, implementation, depth, seam, adapter, leverage, locality. Do not say component, service, API, or boundary for this design talk.

**Incorrect:** a wide bag of optional fields and a thin wrapper that only forwards:

```rust
pub fn compile(opts: CompileOpts) -> Result<Module, Diagnostic> {
    draconic_parser::parse(opts.source.unwrap())
        .and_then(draconic_check::check)
        .map(|c| draconic_ir::lower(&c))
}
```

**Correct:** a small interface that hides the work. Borrow input. Return results.

```rust
pub fn compile_source(source: &str) -> Result<Module, Diagnostic> {
    let checked = check_source(source)?;
    Ok(lower(&checked))
}
```

**Notes.** Deletion test: if deleting the module makes complexity vanish, it was a pass-through. One adapter is a hypothetical seam; two adapters make it real. Do not add a seam until something varies across it. Internal seams can stay private to the module's tests. Frontend (`crates/draconic-frontend/src/lib.rs`) is the model small crate.
