---
title: Native types stay unboxed
impact: CRITICAL
impactDescription: Putting native scalars on the GC heap breaks Dual worlds
tags: [dual, native, gc]
---

## Native types stay unboxed

Native types (`i8`–`i64`, `u8`–`u64`, `f32`/`f64`, `bool`, structs, `*T`) live outside the GC heap. JS values (objects, arrays, strings, closures) are heap-managed.

**Incorrect:** `draconic_rt_alloc_*` for an `i32`, or treating native `bool` as a JS Boolean object.

**Correct:** LLVM lowers native scalars to LLVM integers/floats/pointers. On js, N04 polyfills them as ordinary JS numbers/objects/arrays. Pointers stay native-only.

**Notes.** ADR-0003. Ownership-only and arena-only models were rejected. See `js-n04-polyfill` and `rt-gc-js-values`.
