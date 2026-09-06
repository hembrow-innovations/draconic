---
title: GC heap is JS values
impact: HIGH
impactDescription: Native scalars on the tracing heap collapse Dual worlds
tags: [runtime, gc]
---

## GC heap is JS values

The Runtime tracing GC hosts JS values: `draconic_rt_alloc_string`, `alloc_object`, root push/pop, collect. Native scalars stay LLVM unboxed.

**Incorrect:** allocating a GC object to hold `i32`.

**Correct:** `DraconicValue *` for objects/strings/closures; LLVM `i32`/`i64`/`double`/`ptr` for native types.

**Notes.** ADR-0003. Job queue and Promise ABI live here too. See `dual-native-unboxed`.
