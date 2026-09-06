---
title: Fuzz hooks treat Ok and Err as success
impact: HIGH
impactDescription: Asserting Ok in fuzz hides panic-only bugs
tags: [test, fuzz]
---

## Fuzz hooks treat Ok and Err as success

Designed fuzz entry points return normally on `Ok` and `Err`. Panic is failure. Standalone cargo-fuzz packages stay excluded from the workspace.

**Incorrect:**

```rust
pub fn fuzz_parse(data: &[u8]) {
    let src = String::from_utf8_lossy(data);
    parse(&src).unwrap();
}
```

**Correct:**

```rust
pub fn fuzz_parse(data: &[u8]) {
    let src = String::from_utf8_lossy(data);
    let _ = parse(&src);
}
```

Build the harness with `--manifest-path crates/draconic-parser/fuzz/Cargo.toml`.

**Notes.** Hooks: `draconic_parser::fuzz_parse`, `draconic_embed::fuzz_eval`, `draconic_runtime::fuzz_runtime`. Do not add fuzz packages as workspace members. Full Test262 allowlist is opt-in (`DRACONIC_TEST262_FULL`); default workspace uses a smoke slice.
