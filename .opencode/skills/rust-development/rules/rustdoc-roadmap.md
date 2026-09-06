---
title: Rustdoc cites Roadmap IDs
impact: MEDIUM
impactDescription: Public fns without IDs lose the Loop trail
tags: [rustdoc, roadmap]
---

## Rustdoc cites Roadmap IDs

New public APIs get `///`. Crate roots get `//!` with the Roadmap ID they implement. Short `//` notes lock ECMA early-errors or policy (`// F08.01:`).

**Incorrect:**

```rust
pub fn compile_source(source: &str) -> Result<Module, Diagnostic> {
    // compile it
}
```

**Correct:**

```rust
//! Frontend: compile Draconic source (or an entry path) to IR.

/// Compile `source` as a Script (no filesystem link graph).
pub fn compile_source(source: &str) -> Result<Module, Diagnostic> { /* … */ }
```

**Notes.** Keep rustdoc on new public types and grammar flags. Cite Roadmap IDs, not essays. Product comments exist; they are not banned. See `crate-frontend-entry`.
