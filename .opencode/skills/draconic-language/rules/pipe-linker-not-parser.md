---
title: Linker owns the ESM graph
impact: CRITICAL
impactDescription: Resolving imports in the parser splits the product
tags: [pipe, linker, parser]
---

## Linker owns the ESM graph

The parser turns tokens into an AST. The linker loads an entry's ESM graph, mangles bindings, and flattens to one Program. Frontend chooses parse vs link.

**Incorrect:** resolving `from "./x"` inside `draconic-parser`, or calling the linker a bundler in product copy.

**Correct:** `parse` / `parse_module` for grammar; `link_entry` / `link_entry_with_packages` after Module detection; `draconic-pkg` for git module identity.

**Notes.** CONTEXT.md: Linker is not part of the Parser product. Lower panics on leftover `import`/`export`. See `ir-after-link` and `pkg-git-modules`.
