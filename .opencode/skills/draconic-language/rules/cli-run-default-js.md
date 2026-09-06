---
title: run defaults to js
impact: MEDIUM
impactDescription: Assuming native from draconic run skips LLVM
tags: [cli, run]
---

## run defaults to js

`draconic run file.drac` uses the js target. Native requires `--target native`.

**Incorrect:** assuming `draconic run` links the Runtime binary.

**Correct:**

```bash
draconic run app.drac
draconic run --target native app.drac
```

**Notes.** Shebang `#!/usr/bin/env draconic` is `run`. Targets on CLI are `js` | `native` only. wasm32-wasi is an LLVM API, not `--target`. See `cli-build-requires-target`.
