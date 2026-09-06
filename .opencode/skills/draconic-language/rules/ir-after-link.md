---
title: Lower after link
impact: HIGH
impactDescription: Import statements in IR panic at lower
tags: [ir, linker]
---

## Lower after link

Import/export must be linked before lower. Type aliases are gone. Class lowering is already desugared.

**Incorrect:** `lower` on a Program that still has `Stmt::ImportDeclaration`.

**Correct:** Frontend `compile_path` links when the entry has module syntax, then checks as Module, then lowers. Lower panics with `import/export must be linked before lower` if they remain.

**Notes.** `Stmt::ExternFunction` appears iff `has_extern_ffi`. `body_spans` matches `body` for source maps / DWARF. See `pipe-linker-not-parser`.
