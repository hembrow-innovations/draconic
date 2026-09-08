---
title: scratch build uses .out names
impact: MEDIUM
impactDescription: In-tree emit next to source looks like a fixture and gets committed
tags: [cli, build]
---

## scratch build uses .out names

When `-o` is omitted, `draconic build` writes `{stem}.out.js` (js) or `{stem}.out` (native) beside the input. Gitignore covers `*.out.js`, `*.out.js.map`, `*.out`, and `*.dSYM`. Promise: `toolchain.cli:build-scratch-name`.

**Incorrect:** `draconic build --target js fixture.drac` that you expect to produce `fixture.js`, committing that JS, or leaving a nameless native binary next to a `.drac` fixture.

**Correct:**

```bash
draconic build --target js file.drac
draconic build --target native file.drac
draconic build --target js hello.drac -o hello.js
```

Omit `-o` for throwaway emit (gitignored). Pass `-o` only when the artifact should be kept. Prefer `draconic test` for Conformance; do not drop siblings under `tests/conformance/fixtures/`.

**Notes.** `run` still builds in a temp dir. Explicit `-o hello.js` is a kept file, not scratch. See `cli-build-requires-target`. Product text: `docs/architecture/architecture-cli.md`.
