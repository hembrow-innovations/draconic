---
title: LLVM adapter dispatch
impact: HIGH
impactDescription: Mega-matches in lib.rs hide subsets and blow the file budget
tags: [llvm, adapter]
---

## LLVM adapter dispatch

New native lowering lives in a private adapter: `is_foo_module` plus `emit_foo`. `emit_llvm_ir_raw` dispatches, then falls through to empty hello or unsupported.

**Incorrect:** implementing a feature as another thousand lines inside `emit_llvm_ir_raw`.

**Correct:**

```
// host_tcp.rs
pub(crate) fn is_host_tcp_module(module: &Module) -> bool { … }
pub(crate) fn emit_host_tcp(module: &Module) -> Result<String, Diagnostic> { … }
```

Wire the pair in `lib.rs` dispatch. Copy a neighboring adapter.

**Notes.** See `size-file-budget` and `llvm-no-hello-stub`. Public surface stays `emit_llvm_ir` / `build_native_binary`.
