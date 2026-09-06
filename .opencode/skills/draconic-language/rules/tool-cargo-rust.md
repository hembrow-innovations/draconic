---
title: Cargo and Rust only
impact: CRITICAL
impactDescription: A second host language forks the Compiler
tags: [tool, cargo, rust]
---

## Cargo and Rust only

The Compiler is Rust permanently. Build and test with cargo. Do not self-host in Draconic. Do not add a second host language for Frontend, IR, or backends.

**Incorrect:** a bootstrap milestone or a new crate in Go/Zig/Draconic that owns parse or emit.

**Correct:**

```bash
cargo test -p draconic-frontend
cargo test --workspace
cargo build -p draconic-cli
```

**Notes.** ADR-0001. Workspace members live under `crates/` plus `tests/{conformance,integration,packages,test262}`. Prefer the `draconic` CLI over ad-hoc scripts. Parser fuzz packages stay `exclude`d from the workspace.
