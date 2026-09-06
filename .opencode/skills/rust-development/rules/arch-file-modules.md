---
title: File modules, not mod.rs trees
impact: CRITICAL
impactDescription: Foreign layout fights every neighboring crate
tags: [arch, layout, modules]
---

## File modules, not mod.rs trees

Crates use `mod foo;` plus `foo.rs`. There are no `mod.rs` directories under `crates/*/src`. Nested modules stay private unless re-exported.

**Incorrect:**

```
crates/draconic-pkg/src/
  lock/mod.rs
  lock/entry.rs
  lock/parse.rs
```

**Correct:**

```
crates/draconic-pkg/src/
  lib.rs          # mod lock;
  lock.rs
```

```rust
mod lock;
pub use lock::{LockEntry, LockEntryError};
```

**Notes.** Copy the neighboring tree. A new concern gets a `foo.rs` beside `lib.rs`, not a folder. `draconic-cli` is lib + `src/main.rs` (`[[bin]] name = "draconic"`). Runtime may split `#[cfg(test)]` into `host_abi_tests.rs` still declared from `lib.rs`. See `size-file-budget`.
