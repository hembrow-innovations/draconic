---
id: "test"
title: "Types tests"
kind: test
description: "Which tests cover Checker promises, how, and why."
status: active
domain: draconic
area: types
tags: [test]
created_at: "2026-09-06"
updated_at: "2026-09-06"
---

# Types tests

Purpose: [[specs/draconic/language/purpose]]. Contract: [[specs/draconic/language/types/contract]].

## Coverage

Checker unit tests live in `tests/conformance/tests/types.rs` and `crates/draconic-check/src/lib.rs`. Erase-path and reject fixtures live under `tests/conformance/fixtures/types/`. These lock Checker behaviour, not tsc.

Locks `language.types:annotations`, `language.types:mismatch-diagnostics`, `language.types:structural-shapes`, `language.types:unions-intersections-narrowing`, `language.types:generics`, `language.types:call-site-checking`, `language.types:missing-return`, `language.types:unknown-shape-property`, `language.types:excess-property`, `language.types:annotated-non-callable`.

## Tests

- **tests/conformance/tests/types.rs** — `let_annotation_sets_binding_type`
  - **How:** Parse `let x: number = 1;` and assert the binding type is Number.
  - **Why:** Locks `language.types:annotations`.
- **tests/conformance/tests/types.rs** — `let_annotation_rejects_mismatched_init`
  - **How:** Check `let x: number = "no"` and require a “not assignable” diagnostic naming string and number.
  - **Why:** Locks `language.types:mismatch-diagnostics`.
- **tests/conformance/tests/types.rs** — `object_type_ann_rejects_missing_prop` / `type_alias_object_ok`
  - **How:** Accept a matching shape literal; reject a missing property.
  - **Why:** Locks `language.types:structural-shapes`.
- **tests/conformance/tests/types.rs** — `union_rejects_outside_member` / `typeof_narrow_rejects_wrong_branch`
  - **How:** Reject a boolean in `string | number`; reject using a narrowed string as number.
  - **Why:** Locks `language.types:unions-intersections-narrowing`.
- **tests/conformance/tests/types.rs** — `generic_function_identity_rejects_mismatch`
  - **How:** Infer `id(1)` as number and reject assigning it to string.
  - **Why:** Locks `language.types:generics`.
- **tests/conformance/tests/types.rs** — `call_arg_type_mismatch_errors` / `call_reject_fixtures_run`
  - **How:** Reject wrong arity and wrong argument types; run `types/reject/*` fixtures.
  - **Why:** Locks `language.types:call-site-checking`.
- **crates/draconic-check/src/lib.rs** — `check_missing_return_errors`
  - **How:** Check an annotated non-void function that falls off the end; require “missing return”.
  - **Why:** Locks `language.types:missing-return`. Fixture `types/reject/missing_return` is the Conformance twin.
- **tests/conformance/tests/types.rs** — `unknown_shape_prop_read_errors`
  - **How:** Read `p.y` on `{ x: number }` and require “unknown property”.
  - **Why:** Locks `language.types:unknown-shape-property`.
- **tests/conformance/tests/types.rs** — `excess_prop_annotated_shape_errors`
  - **How:** Assign `{ a: 1, b: 2 }` to `{ a: number }` and require “excess property”.
  - **Why:** Locks `language.types:excess-property`.
- **tests/conformance/tests/types.rs** — `call_annotated_number_errors`
  - **How:** Call an annotated `number` and require “not callable”.
  - **Why:** Locks `language.types:annotated-non-callable`.

## Gaps

- No test yet for promise `language.types:forbid-tsc-compatibility`. ADR-0005 states the rule; nothing titled as “not tsc” or “does not emit TypeScript” locks it.
- Dual-world `as` tests live in [[specs/draconic/language/dual-worlds/test]]. Native integer names live in [[specs/draconic/language/native-types/test]].
