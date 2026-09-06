---
title: Internal crates via workspace.dependencies
impact: CRITICAL
impactDescription: Path drift and version forks split the graph
tags: [tool, cargo, workspace]
---

## Internal crates via workspace.dependencies

Every `draconic-*` crate is declared once under `[workspace.dependencies]` as `{ path = "crates/…" }`. Members take it with `{ workspace = true }`. Shared third-party pins live there too.

**Incorrect:**

```toml
# crates/draconic-cli/Cargo.toml
[dependencies]
draconic-frontend = { path = "../draconic-frontend" }
thiserror = "1"
```

**Correct:**

```toml
# crates/draconic-cli/Cargo.toml
[dependencies]
draconic-frontend = { workspace = true }
```

**Notes.** Crate-local deps are rare (lexer `regress`, `unicode-id-start`). Add a workspace pin before a new shared crate. Test packages live under `tests/` and are workspace members with `publish = false`. Fuzz packages stay excluded. See `crate-members` and `dep-no-extra`.
