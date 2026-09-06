---
id: "contract"
title: "Dual worlds — Contract"
kind: contract
description: "JS values and native types coexist at explicit boundaries. Locked promises carry a test pointer."
status: active
domain: draconic
area: dual-worlds
tags: [contract]
created_at: "2026-09-06"
updated_at: "2026-09-06"
---

# Dual worlds — Contract

Purpose: [[specs/draconic/language/purpose]]. Tests: [[specs/draconic/language/dual-worlds/test]]. Decision: [[0003-gc-runtime-and-dual-worlds]]. Native-type values: [[specs/draconic/language/native-types/contract]]. Glossary: [[CONTEXT]].

## Behaviour

- `language.dual-worlds:explicit-as-boundary`: Crossing between a JS `number` and a native numeric type is an explicit `as` conversion. Same-type `as` is identity.
  test: as_number_to_i32
  test: as_i32_to_number
  test: as_same_type_identity
  test: types/dual/boundary_as
  test: dual_boundary_as_runs
- `language.dual-worlds:forbid-silent-cross-world`: The Checker rejects `as` that is not a dual-worlds-legal hop (string to `i32`, `i32` to string, `i32` to `i64` without a number hop). There is no silent coercion between worlds.
  test: as_rejects_string_to_i32
  test: as_rejects_i32_to_string
  test: as_rejects_i32_to_i64_without_number_hop
  test: native_rejects_number_binding_to_i32
  test: native_rejects_i32_to_number
- `language.dual-worlds:native-only-hard-error-on-js`: A native-only feature (pointers: `*T`, `&x`, `*p`, `*p = v`) is valid on LLVM and hard-errors on the JS backend with a diagnostic. The JS backend does not emit silent wrong code.
  test: native/js-policy/ptr_hard_error
  test: ptr_hard_error_on_js
  test: native/js-policy/ptr_store_hard_error
  test: n04_pointer_hard_error
- `language.dual-worlds:portable-native-scalars`: Native scalars, structs, and fixed arrays that N04 allows are portable: the JS backend polyfills them as ordinary JS values with equivalent observable results.
  test: native/js-policy/scalar_polyfill
  test: scalar_polyfill_on_js
  test: native/js-policy/struct_polyfill
  test: native/js-policy/array_polyfill
- `language.dual-worlds:js-only-hard-error-on-llvm`: A JS-only Program is valid on the JS backend only. The LLVM backend hard-errors with a diagnostic and does not emit silent wrong code.
- `language.dual-worlds:js-values-on-gc-heap`: JS values (objects, closures, cycles) live on the tracing GC heap. Native types stay unboxed and outside that heap.
