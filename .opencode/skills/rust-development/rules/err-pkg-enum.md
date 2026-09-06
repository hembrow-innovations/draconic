---
title: pkg uses focused Error enums
impact: CRITICAL
impactDescription: Mixing Diagnostic into lockfiles hides validation
tags: [err, pkg, enum]
---

## pkg uses focused Error enums

`draconic-pkg` returns hand-written enums with `Display` + `std::error::Error` + `From` wrapping. Constructors take `impl Into<String>` and validate. This is not the compiler `Diagnostic` path.

**Incorrect:**

```rust
pub fn new(path: String) -> Result<LockEntry, Diagnostic> {
    Ok(LockEntry { path, ..todo!() })
}
```

**Correct:**

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LockEntryError {
    InvalidPath { path: String, reason: &'static str },
    InvalidVersion { version: String, reason: &'static str },
}

impl LockEntry {
    pub fn new(
        path: impl Into<String>,
        version: impl Into<String>,
        git_url: impl Into<String>,
        commit_oid: impl Into<String>,
        content_hash: impl Into<String>,
    ) -> Result<Self, LockEntryError> { /* validate */ }
}
```

**Notes.** Copy `LockEntryError`, `ManifestError`, `GetError`. Wrap with `From` at crate edges (`ManifestError` → `GetError`). Do not start using thiserror here unless neighbors already do. See `btreemap-lock`.
