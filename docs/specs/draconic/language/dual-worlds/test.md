---
id: "test"
title: "Dual worlds tests"
kind: test
description: "Which tests cover dual-worlds boundaries, how, and why."
status: active
domain: draconic
area: dual-worlds
tags: [test]
created_at: "2026-09-06"
updated_at: "2026-09-06"
---

# Dual worlds tests

Purpose: [[specs/draconic/language/purpose]]. Contract: [[specs/draconic/language/dual-worlds/contract]].

## Coverage

High-risk locks: miscompile across worlds, and native-only features emitting wrong JS. Checker tests in `tests/conformance/tests/types.rs`; JS-policy fixtures under `tests/conformance/fixtures/native/js-policy/`; emit tests in `crates/draconic-backend-js`.

Locks `language.dual-worlds:explicit-as-boundary`, `language.dual-worlds:forbid-silent-cross-world`, `language.dual-worlds:native-only-hard-error-on-js`, `language.dual-worlds:portable-native-scalars`.

## Tests

- **tests/conformance/tests/types.rs** — `as_number_to_i32` / `dual_boundary_as_runs`
  - **How:** Check `n as i32` / `x as number`; run fixture `types/dual/boundary_as` on js and native.
  - **Why:** Locks `language.dual-worlds:explicit-as-boundary`. Crossing is `as`, not inference.
- **tests/conformance/tests/types.rs** — `as_rejects_string_to_i32`
  - **How:** Check `s as i32` for a string and require a diagnostic containing `dual-worlds`.
  - **Why:** Locks `language.dual-worlds:forbid-silent-cross-world`. Same for i32→string and i32→i64 without a number hop.
- **tests/conformance/tests/native_js_policy.rs** — `ptr_hard_error_on_js`
  - **How:** Run fixture `native/js-policy/ptr_hard_error` on js; meta expects compile/emit failure.
  - **Why:** Locks `language.dual-worlds:native-only-hard-error-on-js`. Twin crate test `n04_pointer_hard_error`.
- **tests/conformance/tests/native_js_policy.rs** — `scalar_polyfill_on_js`
  - **How:** Run `native/js-policy/scalar_polyfill` on js and assert program results.
  - **Why:** Locks `language.dual-worlds:portable-native-scalars`. Struct and array polyfill fixtures are siblings.

## Gaps

- No test yet for promise `language.dual-worlds:js-only-hard-error-on-llvm` in this language slice. Host I/O js-only/native-only fixtures belong to other spec trees and are not pointers here.
- No test yet for promise `language.dual-worlds:js-values-on-gc-heap` (tracing GC vs unboxed native). ADR-0003 states it; Runtime GC hello is toolchain, not a dual-worlds miscompile lock.
