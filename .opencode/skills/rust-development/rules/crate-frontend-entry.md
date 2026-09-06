---
title: Callers use draconic-frontend
impact: CRITICAL
impactDescription: Rewired stages fork Script vs Module policy
tags: [crate, frontend, pipeline]
---

## Callers use draconic-frontend

Product callers compile through `draconic-frontend`. That crate owns Script vs Module (link) policy, then check → lower. CLI, backends' tests, embed, and conformance enter here.

**Incorrect:**

```rust
let program = draconic_parser::parse(source)?;
let checked = draconic_check::check(program)?;
let module = draconic_ir::lower(&checked);
```

**Correct:**

```rust
use draconic_frontend::{check_source, compile_path, compile_source};

let module = compile_source(source)?;
let module = compile_path(entry)?;
let checked = check_source(source)?;
```

**Notes.** `compile_source` is Script (embed, single buffer). `compile_path` is parse-driven Module link when the entry has import/export. `compile_source_module` is the Module goal without a filesystem graph. Stage crates stay public for their own tests and for frontend. See `pipe-stages` and `pipe-script-module`.
