---
id: "contract"
title: "Exceptions — Contract"
kind: contract
description: "Catchable exceptions versus process abort. Locked promises carry a test pointer."
status: active
domain: draconic
area: exceptions
tags: [contract]
created_at: "2026-09-06"
updated_at: "2026-09-06"
---

# Exceptions — Contract

Purpose: [[specs/draconic/language/purpose]]. Tests: [[specs/draconic/language/exceptions/test]]. Decision: [[0011-catchable-exceptions-vs-abort]]. Glossary: [[CONTEXT]].

## Behaviour

- `language.exceptions:throw-try-catch`: User `throw` of a JS value is a catchable exception. `try`/`catch` binds the thrown value (including nested rethrow) and does not abort the process.
  test: es/exceptions/throw_try_catch
  test: throw_try_catch_runs
  test: security/panic_policy/catchable_exceptions
  test: catchable_exceptions_runs_native
- `language.exceptions:finally`: `finally` always runs for `try`/`catch`/`finally` and `try`/`finally`. Completion after `finally` follows ECMA meaning.
  test: es/exceptions/try_finally
  test: try_finally_runs
- `language.exceptions:optional-catch-binding`: `catch { … }` without a parameter is legal, with or without `finally`.
  test: es/exceptions/optional_catch
  test: optional_catch_runs
- `language.exceptions:catch-destructure`: A `catch` binding may be a destructuring pattern.
  test: es/exceptions/catch_destructure
  test: catch_destructure_runs
- `language.exceptions:catchable-continues`: After `catch`, execution continues. Catchable exceptions never become process abort.
  test: catchable_exceptions_runs_native
  test: security/panic_policy/catchable_exceptions
- `language.exceptions:abort-not-catchable`: Process abort (`draconic_rt_abort` and Runtime invariant failure) is not a JS value. `try`/`catch` does not run. After abort the process is dead (non-zero exit, no after-abort stdout).
  test: security/panic_policy/abort_process
  test: abort_process_kills_native
- `language.exceptions:ecma-builtin-throws-catchable`: Language operations that throw TypeError / RangeError / ReferenceError (and user `throw new TypeError(…)`) are catchable exceptions, not abort.
- `language.exceptions:uncaught-nonzero`: An uncaught catchable exception still terminates the Program with a non-zero exit. It remains the catchable class, just not handled.
