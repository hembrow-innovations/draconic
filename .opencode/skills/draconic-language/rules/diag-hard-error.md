---
title: Hard-error beats wrong code
impact: CRITICAL
impactDescription: Silent subsetting ships a Program the other backend cannot mean
tags: [diag, hard-error, portable]
---

## Hard-error beats wrong code

Native-only and JS-only features hard-error with a diagnostic on the other backend. Never emit wrong code. Never succeed via the B08 hello stub.

**Incorrect:** JS emit of `&x` as a no-op; LLVM success that prints `hello\n` for `let o = {}`.

**Correct:** `js.error: native-only` for pointers; `unsupported_native_diagnostic()` for unclassified LLVM IR. Portable programs run on both backends after documented polyfills.

**Notes.** CONTEXT.md: Native-only / JS-only. Conformance meta: `js.error:` / `native.error:` plus optional `js.error_code:`. See `js-n04-polyfill` and `llvm-no-hello-stub`.
