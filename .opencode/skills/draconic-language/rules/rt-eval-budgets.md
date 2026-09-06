---
title: Eval budgets fail closed
impact: HIGH
impactDescription: Catchable OOM in Embed turns a limit into a Program value
tags: [runtime, embed, budget]
---

## Eval budgets fail closed

Embed size/alloc/time limits fail closed. They are diagnostics or C `NULL`, not catchable JS exceptions.

**Incorrect:** `try { eval(huge) } catch (e) { … }` succeeding as a language Error when the alloc budget trips.

**Correct:** `eval_source_with_limits` returns a diagnostic; C ABI sets `NULL` / exceeded flag; if the Runtime cannot continue, abort.

**Notes.** R01.02 / R01.03. Embed is a subset interpreter; grow it with N07 fixtures rather than assuming full ES. See `pipe-embed-source`.
