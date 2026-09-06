---
title: Hello stub is empty-program only
impact: CRITICAL
impactDescription: Stub-green native fixtures fake N08 observations
tags: [llvm, hello, native]
---

## Hello stub is empty-program only

B08 `draconic_rt_hello` is the empty Program only. Non-empty unsupported IR returns `unsupported_native_diagnostic()`. Native fixtures assert program results, not `hello\n`.

**Incorrect:** `emit_llvm_ir` success that calls `draconic_rt_hello` for `let o = {}`.

**Correct:** classify the subset (`is_*_module`) and emit it, or `Err`. Empty body → hello. Anything else unsupported → diagnostic.

**Notes.** ROADMAP native observations: `native.stdout` is program output. See `llvm-adapter-dispatch` and `test-native-observations`.
