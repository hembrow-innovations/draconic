---
title: Do not introduce Cow unmeasured
impact: LOW
impactDescription: Cow clones the API for a maybe-borrow
tags: [avoid, cow, ownership]
---

## Do not introduce Cow unmeasured

`Cow` is unused. Borrow in, own out. Do not add `Cow<'_, str>` to public compiler fns without a measured need.

**Incorrect:**

```rust
pub fn compile_source(source: Cow<'_, str>) -> Result<Module, Diagnostic>
```

**Correct:**

```rust
pub fn compile_source(source: &str) -> Result<Module, Diagnostic>
```

**Notes.** See `api-borrow-own` and `own-box-ast`.
