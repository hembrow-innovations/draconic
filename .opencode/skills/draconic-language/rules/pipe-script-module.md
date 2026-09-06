---
title: Script vs Module is parse-driven
impact: CRITICAL
impactDescription: Substring heuristics mis-classify comments and identifiers
tags: [pipe, module, parse]
---

## Script vs Module is parse-driven

Module detection looks at the AST for `ImportDeclaration` / `Export*` statements. Source text is not the policy.

**Incorrect:**

```rust
if source.contains("import") || source.contains("export") {
    link_entry(path)
}
```

**Correct:** parse as Script first; if the AST has module syntax, `link_entry`. If Script parse fails, retry `parse_module` and link only when that AST has module syntax. Frontend's `load_program` already does this.

**Notes.** `let import_name = 1` is Script. ESM files in the linker always `parse_module`. JSON files become a synthetic default-export module. See `pipe-linker-not-parser`.
