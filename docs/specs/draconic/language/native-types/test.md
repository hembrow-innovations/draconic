---
id: "test"
title: "Native types tests"
kind: test
description: "Which tests cover unboxed native types, how, and why."
status: active
domain: draconic
area: native-types
tags: [test]
created_at: "2026-09-06"
updated_at: "2026-09-06"
---

# Native types tests

Purpose: [[specs/draconic/language/purpose]]. Contract: [[specs/draconic/language/native-types/contract]].

## Coverage

Native observations under `tests/conformance/fixtures/native/` plus Checker tests that native types are not JS `number`. JS-policy (hard-error vs polyfill) is mapped in [[specs/draconic/language/dual-worlds/test]].

Locks `language.native-types:integers`, `language.native-types:floats-bool`, `language.native-types:structs`, `language.native-types:fixed-arrays`, `language.native-types:pointers`, `language.native-types:not-js-primitives`.

## Tests

- **tests/conformance/tests/native_ints.rs** — `arith_i32_runs_native`
  - **How:** Run `native/ints/arith_i32` on native and assert program stdout, not the hello stub.
  - **Why:** Locks `language.native-types:integers`. Widths and bitwise are sibling fixtures.
- **tests/conformance/tests/types.rs** — `i32_annotation_sets_binding_type` / `native_integer_and_float_names`
  - **How:** Check `let x: i32 = 1` and the full integer/float name set; assert `Type::Native`.
  - **Why:** Locks that native names are Checker types, not JS primitives.
- **tests/conformance/tests/native_floats.rs** — `arith_f64_runs_native`
  - **How:** Run `native/floats/arith_f64` on native.
  - **Why:** Locks `language.native-types:floats-bool`.
- **tests/conformance/tests/native_layout.rs** — `struct_basic_runs_native`
  - **How:** Run `native/layout/struct_basic` on native.
  - **Why:** Locks `language.native-types:structs`.
- **tests/conformance/tests/native_layout.rs** — `array_basic_runs_native`
  - **How:** Run `native/layout/array_basic` on native.
  - **Why:** Locks `language.native-types:fixed-arrays`.
- **tests/conformance/tests/native_layout.rs** — `ptr_basic_runs_native`
  - **How:** Run `native/layout/ptr_basic` on native.
  - **Why:** Locks `language.native-types:pointers`.
- **tests/conformance/tests/types.rs** — `native_rejects_number_binding_to_i32`
  - **How:** Assign a `number` binding to `i32` without `as`; require not-assignable naming both types.
  - **Why:** Locks `language.native-types:not-js-primitives`.

## Gaps

- No test yet for promise `language.native-types:outside-gc-heap`. ADR-0003 requires unboxed native values off the GC heap; no `#[test]` title or fixture id says that.
