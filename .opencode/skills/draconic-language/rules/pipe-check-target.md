---
title: Check is target-neutral by default
impact: CRITICAL
impactDescription: Assuming check rejects native-only host APIs hides emit errors
tags: [pipe, check, target]
---

## Check is target-neutral by default

`check_path` / `check_source` do not pass `CompileTarget`. Native-only and JS-only rejection also happens at emit. `draconic check` can accept a Program that `build --target js` will hard-error.

**Incorrect:** assuming `check_path` fails on `*i32` or `tcpListen` for the js target.

**Correct:** use `check_for_target` / `check_module_for_target` when the backend is known. CLI `build`/`run` still hard-error at JS or LLVM emit. Conformance `js.error:` asserts the emit diagnostic.

**Notes.** Host APIs are free globals; unavailable on a target → E0400 at the target-aware check or at emit. See `diag-hard-error` and `dual-host-globals`.
