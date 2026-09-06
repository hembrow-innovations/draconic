---
title: Language bar is conformance fixtures
impact: CRITICAL
impactDescription: Unit-only language tests miss both backends
tags: [test, conformance, fixtures]
---

## Language bar is conformance fixtures

Language semantics live under `tests/conformance`: `*.drac` plus `*.meta` sidecar. Thin `tests/conformance/tests/<area>.rs` files call `load_fixtures` / `run_fixture`.

**Incorrect:** asserting JS emit strings in `draconic-backend-js` as the only proof an arrow function works.

**Correct:**

```
tests/conformance/fixtures/es/functions/arrow.drac
tests/conformance/fixtures/es/functions/arrow.meta
```

```text
id: es/functions/arrow
targets: js,native
js.exit: 0
native.exit: 0
native.stdout: 42\n3\n42\n42\n0\n17\n
js.check: if (a !== 42) process.exit(1); …
```

**Notes.** `.drac` without `.meta` is a dependency module unless it looks like `describe(` / `it(`. Native observations are real stdout. JS-only native features use `error_contains` / `error_code`. Copy a neighboring fixture. See `test-both-targets`.
