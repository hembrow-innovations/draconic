---
title: Conformance fixtures are the Done bar
impact: HIGH
impactDescription: Unit tests alone cannot mark a language item done
tags: [test, conformance]
---

## Conformance fixtures are the Done bar

Language semantics pin in `tests/conformance/fixtures/**/*.drac`. A Roadmap item is done when its Tests paths are green on every applicable target.

**Incorrect:** crate unit tests only, then `ROADMAP.md` → `done`.

**Correct:** add `.drac` + `.meta` under the area folder (`es/`, `types/`, `native/`, `host/`, …). Area tests load fixtures via `run_fixture`. Compile always uses `compile_path` so modules link.

**Notes.** Prefer crate unit tests for compiler pieces; Conformance for language meaning; both backends when Targets is `both`. Load **tdd** / **draconic-loop** for the red-green sitting. See `test-meta-sidecar`.
