---
title: build requires --target
impact: MEDIUM
impactDescription: A target-less build has no backend
tags: [cli, build]
---

## build requires --target

`draconic build` needs `--target js` or `--target native`. `parse` dumps AST. `check` binds and typechecks without emit.

**Incorrect:** `draconic build file.drac` or treating `parse` as typecheck.

**Correct:**

```bash
draconic build --target js file.drac -o out.js
draconic build --target native file.drac -o out.bin
draconic check file.drac
```

**Notes.** `--strip` and `--lto` are native-only. `--offline` is package cache only. `--watch` exists on check/build.
