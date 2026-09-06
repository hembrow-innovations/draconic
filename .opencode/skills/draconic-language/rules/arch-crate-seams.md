---
title: Enter at Frontend
impact: CRITICAL
impactDescription: Skipping crates forks Script vs Module policy
tags: [arch, frontend, seam]
---

## Enter at Frontend

Callers that compile a Program enter at `draconic-frontend`. That crate owns Script vs Module (link) policy, then check, then lower. CLI, Embed, and LSP do not wire stage crates for a normal compile.

**Incorrect:**

```rust
let program = draconic_parser::parse(&src)?;
let checked = draconic_check::check(program)?;
let module = draconic_ir::lower(&checked);
```

**Correct:**

```rust
use draconic_frontend::{compile_path, compile_source, check_path};

let ir = compile_path(entry)?;
let ir = compile_source(src)?; // Embed / no filesystem
let checked = check_path(entry)?; // no lower
```

**Notes.** `draconic parse` is the exception: it dumps AST via `parse_and_dump`. Linker is not a caller-facing product; Frontend chooses parse vs `link_entry`. Backends consume IR only. See `pipe-compile-path`.
