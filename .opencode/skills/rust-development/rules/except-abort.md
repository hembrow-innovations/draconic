---
title: Catchable exceptions are not abort
impact: HIGH
impactDescription: Mapping throw to draconic_rt_abort kills try/catch
tags: [except, abort, runtime]
---

## Catchable exceptions are not abort

A **Catchable exception** is a JS-value failure a Program can handle with `try`/`catch`. Process abort is `draconic_rt_abort` (R04.02). GC / OOM / abort are not `try`/`catch`.

**Incorrect:**

```c
void throw_js(value v) { draconic_rt_abort("throw"); }
```

**Correct:** user `throw` and ECMA `Error` objects stay in the JS exception path. `draconic_rt_abort` is for invariant / permission-to-die failures.

**Notes.** ADR-0011 / Roadmap R04.01 vs R04.02. Vocabulary in `CONTEXT.md`. Embed budget failures are diagnostics, not catchable JS (`embed-eval`).
