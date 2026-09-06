---
title: Real spans on Program errors
impact: CRITICAL
impactDescription: Dummy spans hide the offending expression
tags: [diag, span]
---

## Real spans on Program errors

Type and syntax errors use the expression or binding span. `Span::dummy()` is for IO and backend-wide failures that have no source range.

**Incorrect:** `Diagnostic::new("not assignable", Span::dummy())` on a bad `let`.

**Correct:** pass the AST node's span. Dummy is reserved for read failures, missing libs, and "unsupported IR" as a whole module.

**Notes.** `Span` is a half-open UTF-8 byte range. `Span::dummy()` is `0..0`. See `diag-codes`.
