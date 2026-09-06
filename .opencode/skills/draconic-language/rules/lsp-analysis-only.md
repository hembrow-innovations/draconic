---
title: LSP is analysis
impact: MEDIUM
impactDescription: A tower-lsp rewrite stalls hover and goto
tags: [lsp, analysis]
---

## LSP is analysis

`draconic-lsp` is `Analysis::analyze`, hover, and goto_definition over `check_source`. It is not a full language server product.

**Incorrect:** adding tower-lsp JSON-RPC as the first change when the task is hover.

**Correct:** keep analysis on Frontend `check_source` (Script). Grow hover/goto at that seam.

**Notes.** CLI `check --watch` is a different surface. See `pipe-compile-path`.
