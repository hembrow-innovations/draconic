---
title: Compile through Frontend entries
impact: CRITICAL
impactDescription: Re-assembling stages drifts Script vs Module semantics
tags: [pipe, frontend, compile]
---

## Compile through Frontend entries

Pick the Frontend entry that matches the input. Filesystem entries use `compile_path` / `check_path`. Buffers with no graph use `compile_source` (Script) or `compile_source_module` (Module, no link).

**Incorrect:** `compile_source` on a path that has `import`, or `compile_path` inside Embed eval.

**Correct:**

```rust
compile_path(entry)?;                 // CLI build/run; links if module syntax
check_path(entry)?;                   // CLI check; no lower
compile_source(src)?;                 // Embed / single buffer; Script
compile_source_module(src)?;          // buffer Module goal; still no link graph
```

**Notes.** Relative imports are unresolved without `compile_path`. Top-level `await` is Script-illegal. See `pipe-script-module` and `pipe-embed-source`.
