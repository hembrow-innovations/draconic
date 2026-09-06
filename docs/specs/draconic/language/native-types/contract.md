---
id: "contract"
title: "Native types — Contract"
kind: contract
description: "Unboxed systems types are not JS primitives. Locked promises carry a test pointer."
status: active
domain: draconic
area: native-types
tags: [contract]
created_at: "2026-09-06"
updated_at: "2026-09-06"
---

# Native types — Contract

Purpose: [[specs/draconic/language/purpose]]. Tests: [[specs/draconic/language/native-types/test]]. Dual-worlds boundaries: [[specs/draconic/language/dual-worlds/contract]]. Checker names: [[specs/draconic/language/types/contract]]. Decision: [[0003-gc-runtime-and-dual-worlds]]. Glossary: [[CONTEXT]].

## Behaviour

- `language.native-types:integers`: A Program uses native integer types `i8`–`i64` and `u8`–`u64` as unboxed values: same-width arithmetic, bitwise ops, compares, updates, and calls on the native target.
  test: native/ints/arith_i32
  test: arith_i32_runs_native
  test: native/ints/widths
  test: native/ints/bitwise
  test: i32_annotation_sets_binding_type
  test: native_integer_and_float_names
- `language.native-types:floats-bool`: A Program uses native `f32`/`f64` and native bool as unboxed values on the native target.
  test: native/floats/arith_f64
  test: arith_f64_runs_native
  test: native/floats/bool_basic
- `language.native-types:structs`: A Program declares a native struct as a type alias of native scalar fields, initializes with an object literal, and reads fields on the native target.
  test: native/layout/struct_basic
  test: struct_basic_runs_native
  test: native/layout/struct_fields
- `language.native-types:fixed-arrays`: A Program declares a fixed native array as a tuple type of native scalars, initializes with an array literal, and reads a const index on the native target.
  test: native/layout/array_basic
  test: array_basic_runs_native
- `language.native-types:pointers`: A Program uses native pointers `*T` (T a native scalar): address-of `&x`, deref `*p`, and store `*p = v` on the native target.
  test: native/layout/ptr_basic
  test: ptr_basic_runs_native
  test: native/layout/ptr_store
- `language.native-types:not-js-primitives`: A native type is not a JavaScript language type. `number` is not assignable to `i32` and `i32` is not assignable to `number` without an explicit dual-worlds `as`. Distinct native widths are not interchangeable.
  test: native_rejects_number_binding_to_i32
  test: native_rejects_i32_to_number
  test: native_rejects_mismatched_native_width
- `language.native-types:outside-gc-heap`: Native types stay unboxed and outside the GC heap that holds JS values.
