---
title: cargo and edition 2021 only
impact: CRITICAL
impactDescription: Mixed toolchains and editions fork the tree
tags: [tool, cargo, edition]
---

## cargo and edition 2021 only

Build and test with cargo. Workspace package edition is 2021. Product source is Rust (`.rs`). Do not add a second package manager, edition, or host language for the compiler.

**Incorrect:**

```bash
python compile.py
node scripts/emit-ir.mjs
```

```toml
[package]
edition = "2024"
```

**Correct:**

```bash
cargo test -p draconic-frontend
cargo test --workspace
cargo build -p draconic-cli
```

**Notes.** Workspace `edition = "2021"` is the pin. Generated or vendored files are not product source. Node may run emitted JS and a few repo scripts (`fetch-test262.mjs`); that is not the toolchain. See `avoid-self-host`.
