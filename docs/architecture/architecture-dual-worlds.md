---
id: "architecture-dual-worlds"
title: "Dual worlds at the native boundary"
kind: architecture
description: "JS values on the GC heap and unboxed native types in one Program, with explicit as-casts at check and LLVM lowering."
domain: draconic
area: runtime
tags: [runtime]
created_at: "2026-09-06"
updated_at: "2026-09-06"
---

# Dual worlds at the native boundary

## Overview

Dual worlds is the coexistence of JS values and native types in one Program, with explicit boundaries at the type and lowering level. It is not an FFI-only model and not “just typed JS.” Glossary: [[CONTEXT]]. Heap implementation: [[system-design-gc-runtime]]. Runtime ABI: [[architecture-runtime]]. Backends: [[architecture-backend-llvm]], [[architecture-backend-js]]. Decision: [[0003-gc-runtime-and-dual-worlds]].

## Context

A Program may use JavaScript objects, arrays, strings, closures, and Promises (heap-managed, JavaScript semantics) and also `i32`, `i64`, fixed structs, and pointers (static, unboxed, not JavaScript language types). [[0003-gc-runtime-and-dual-worlds]] requires a tracing GC for the JS side so cycles and identity match ECMA-262. Ownership-only and arena-only models were rejected because they cut those semantics.

The Checker (T06) and LLVM native-scalar lowering (N08.17) are the type and emit boundaries. The Runtime C heap never stores unboxed native integers as GC objects.

## Design

### Two universes

- **JS value**: heap-managed; objects, arrays, strings, closures, Promises, and other ECMA values. On native, heap objects are `DraconicValue *` in [[architecture-runtime]] (tags string, object, promise, array).
- **Native type**: unboxed systems type — `i8`–`i64`, `u8`–`u64`, `f32`/`f64`, native `bool`, native-field structs, fixed arrays of native scalars, pointers `*T`. LLVM maps these to `iN` / `float` / `double` / `i1` / struct layout / `ptr`. They are not allocated by `draconic_rt_alloc_*`.

### Checker boundary

Implicit conversion across the worlds is rejected (`cannot convert type … across dual-worlds boundary`). The allowed explicit boundary (`as`) is JS `number` ↔ unboxed native numeric (not `bool`). Numeric literals may contextually type as native numbers. Native structs and tuples take object/array literals when every field/element is native.

`bool` is not on that `as` numeric boundary. JS-only features on the JS backend stay JS; native-only FFI/`extern "C"` hard-errors on JS (N04 spirit). Host I/O is a third surface ([[architecture-runtime]]), not Dual-worlds scalars.

### LLVM native boundary

In a native-scalar module, JS `number` lowers as IEEE-754 **unboxed double**, not a GC box. JS `boolean` in that context lowers as `i1`. Width changes between unboxed scalars use explicit int↔float casts. Values that are objects, arrays, heap strings, or Promises go through Runtime GC calls from [[architecture-backend-llvm]].

So “JS number” on native is still a JS-typed value in the Checker, but it is not a `DraconicValue` unless boxed into an object/array property. Dual worlds is a type-and-lowering rule, not “everything JavaScript-shaped lives on the GC.”

### Runtime heap vs stack/registers

GC roots are a growable stack of `DraconicValue *`. Unboxed locals live in LLVM registers/allocas. Mixing a native `i32` into a JS object property requires an explicit boundary (and a box) — there is no silent tag on the C heap for native structs.

Catchable exceptions are JS values ([[0011-catchable-exceptions-vs-abort]]). Process abort and GC OOM are not Dual-worlds conversions; they never become heap values.

## Trade-offs

Unboxed natives give predictable layout and FFI ([[architecture-runtime]] C ABI call targets) without write barriers. Tracing GC gives cycles and JS identity. The explicit `as` boundary prevents tsc-style erasure from smuggling native types through JS object shapes ([[0005-ts-inspired-not-tsc]] is the Checker stance; Dual worlds is why erasure cannot be the model). Cost: two representations for “a number” (JS `number` as `double` vs `i32`/`i64`) and a hard error when the worlds are mixed without `as`.

## Consequences

Conformance for the boundary is `tests/conformance` fixtures `types/dual` (T06 / N08.17). Portable programs must not assume a native type is a JS object. The JS backend has no GC Runtime; native types there polyfill or hard-error per N04. Embed eval ([[architecture-embed]]) currently returns only `EmbedValue` primitives — not native structs and not GC objects. Pipeline placement: [[architecture-pipeline]].
