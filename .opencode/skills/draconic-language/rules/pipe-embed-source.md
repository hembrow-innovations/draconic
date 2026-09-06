---
title: Embed compiles a Script buffer
impact: CRITICAL
impactDescription: Eval has no filesystem link graph
tags: [pipe, embed, eval]
---

## Embed compiles a Script buffer

Embed (`eval`, `new Function`) compiles a string through `compile_source`. It has no entry path and does not resolve relative imports.

**Incorrect:** `compile_path` inside eval, or treating budget exhaustion as a `try`/`catch` Error.

**Correct:** `draconic_embed::eval_source` / `eval_source_with_limits` → Frontend `compile_source` (Script) → subset interpreter. Grow the subset with N07 fixtures.

**Notes.** Top-level `await` is rejected in Script. ADR-0004: native Embed is required for a full superset; JS-backend-only eval was rejected. See `rt-eval-budgets`.
