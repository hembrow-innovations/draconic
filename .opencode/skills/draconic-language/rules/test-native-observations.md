---
title: Native observations are program results
impact: HIGH
impactDescription: Asserting hello\n marks LLVM stub success as language done
tags: [test, native, n08]
---

## Native observations are program results

`Targets: native` / `both` means fixtures assert program results (`native.stdout` / equivalent), not B08 `hello\n`.

**Incorrect:**

```text
targets: native
native.stdout: hello
```

for an ES or native-type Program.

**Correct:** the Program prints or exits with the feature's result. Empty Program is the only hello case.

**Notes.** ROADMAP legend. See `llvm-no-hello-stub`.
