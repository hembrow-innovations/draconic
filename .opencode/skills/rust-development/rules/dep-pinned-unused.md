---
title: Pinned unused crates stay idle
impact: MEDIUM
impactDescription: Growing thiserror into product crates splits error styles
tags: [dep, thiserror, serde]
---

## Pinned unused crates stay idle

Workspace pins `thiserror`, `serde`, and `rayon`. They are unused in product `.rs` (`rayon` is test262 only). Do not grow them into compiler crates just because they are pinned.

**Incorrect:**

```rust
#[derive(thiserror::Error, Debug)]
pub enum CheckError { /* … */ }
```

**Correct:** `Result<T, Diagnostic>` on the compiler path; hand-written pkg enums; rayon only in test262.

**Notes.** `toml` is used as `toml::Value` in pkg, not via serde. `sha2` / `aes-gcm` / `flate2` (rust_backend) belong to the LLVM backend. See `err-diagnostic` and `err-pkg-enum`.
