---
title: Catchable exceptions vs abort
impact: HIGH
impactDescription: Throwing JS errors from GC failure makes abort catchable
tags: [runtime, exception, abort]
---

## Catchable exceptions vs abort

Language failures a Program can handle with `try`/`catch` are catchable exceptions. Runtime invariant failure and budget exhaustion abort the process.

**Incorrect:** `throw` a JS Error on GC root underflow, or `draconic_rt_abort` for `throw new TypeError()`.

**Correct:**

- Catchable: user `throw`, ECMA Error objects, language TypeError/RangeError/…
- Abort: `draconic_rt_abort` (stderr `draconic_rt: abort`, then libc `abort()`)

Uncaught catchable still exits non-zero; the process is not aborted.

**Notes.** ADR-0011 / R04.01 / R04.02. See `rt-eval-budgets`.
