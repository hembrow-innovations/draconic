---
id: "test"
title: "Exceptions tests"
kind: test
description: "Which tests cover catchable exceptions versus abort, how, and why."
status: active
domain: draconic
area: exceptions
tags: [test]
created_at: "2026-09-06"
updated_at: "2026-09-06"
---

# Exceptions tests

Purpose: [[specs/draconic/language/purpose]]. Contract: [[specs/draconic/language/exceptions/contract]].

## Coverage

E10 fixtures under `tests/conformance/fixtures/es/exceptions/` lock try/catch/finally syntax. R04 / ADR-0011 fixtures under `tests/conformance/fixtures/security/panic_policy/` lock catchable vs abort. Prefer these high-risk locks.

Locks `language.exceptions:throw-try-catch`, `language.exceptions:finally`, `language.exceptions:optional-catch-binding`, `language.exceptions:catch-destructure`, `language.exceptions:catchable-continues`, `language.exceptions:abort-not-catchable`.

## Tests

- **tests/conformance/tests/exceptions.rs** — `throw_try_catch_runs`
  - **How:** Run `es/exceptions/throw_try_catch` (throw number/string/from-call, nested rethrow) on declared targets.
  - **Why:** Locks `language.exceptions:throw-try-catch`.
- **tests/conformance/tests/exceptions.rs** — `try_finally_runs`
  - **How:** Run `es/exceptions/try_finally`.
  - **Why:** Locks `language.exceptions:finally`.
- **tests/conformance/tests/exceptions.rs** — `optional_catch_runs`
  - **How:** Run `es/exceptions/optional_catch`.
  - **Why:** Locks `language.exceptions:optional-catch-binding`.
- **tests/conformance/tests/exceptions.rs** — `catch_destructure_runs`
  - **How:** Run `es/exceptions/catch_destructure`.
  - **Why:** Locks `language.exceptions:catch-destructure`.
- **tests/conformance/tests/panic_policy.rs** — `catchable_exceptions_runs_native`
  - **How:** Run `security/panic_policy/catchable_exceptions` on native; require native.stdout and continued execution after catch.
  - **Why:** Locks `language.exceptions:catchable-continues`. User throw is not abort.
- **tests/conformance/tests/panic_policy.rs** — `abort_process_kills_native`
  - **How:** Run `security/panic_policy/abort_process` (`draconic_rt_abort`) on native; expect non-zero exit and no stdout so catch/after cannot have run.
  - **Why:** Locks `language.exceptions:abort-not-catchable`.

## Gaps

- No test yet for promise `language.exceptions:ecma-builtin-throws-catchable`. ADR-0011 lists TypeError/RangeError/ReferenceError from language operations; E10 fixtures throw numbers and strings, not those built-ins.
- No test yet for promise `language.exceptions:uncaught-nonzero`. ADR-0011 says uncaught catchable still exits non-zero; no fixture title locks that path.
