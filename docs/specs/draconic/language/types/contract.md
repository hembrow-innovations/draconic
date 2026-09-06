---
id: "contract"
title: "Types — Contract"
kind: contract
description: "Checker promises: TypeScript-inspired, not tsc. Locked promises carry a test pointer."
status: active
domain: draconic
area: types
tags: [contract]
created_at: "2026-09-06"
updated_at: "2026-09-06"
---

# Types — Contract

Purpose: [[specs/draconic/language/purpose]]. Tests: [[specs/draconic/language/types/test]]. Decision: [[0005-ts-inspired-not-tsc]]. Native-type names in the Checker also appear in [[specs/draconic/language/native-types/contract]]. Dual-world `as` lives in [[specs/draconic/language/dual-worlds/contract]].

## Behaviour

- `language.types:annotations`: The Checker accepts type annotations on bindings, parameters, and function/arrow returns, and assigns those types to the annotated names.
  test: let_annotation_sets_binding_type
  test: param_annotation_sets_param_type
  test: types/annotations_erase
  test: annotations_erase_runs
- `language.types:mismatch-diagnostics`: When an initializer, assignment, default, or return does not match an annotation, the Checker rejects the Program with a diagnostic. It does not erase-only and succeed.
  test: let_annotation_rejects_mismatched_init
  test: let_annotation_rejects_mismatched_assign
  test: return_type_rejects_mismatch
- `language.types:structural-shapes`: The Checker treats object type annotations and `type` aliases as structural shapes: matching literals assign; missing or wrong properties do not.
  test: object_type_ann_accepts_matching_literal
  test: object_type_ann_rejects_missing_prop
  test: type_alias_object_ok
  test: types/object_types_erase
- `language.types:unions-intersections-narrowing`: The Checker accepts union and intersection types and narrows unions on `typeof` checks.
  test: union_accepts_each_member
  test: union_rejects_outside_member
  test: intersection_object_merge
  test: typeof_narrow_string_branch
  test: typeof_narrow_rejects_wrong_branch
  test: types/union_intersection_erase
- `language.types:generics`: The Checker accepts generic type aliases and generic functions, infers type arguments at call sites, and rejects arity or argument mismatches.
  test: generic_type_alias_app
  test: generic_function_identity_infers
  test: generic_function_identity_rejects_mismatch
  test: types/generics_erase
- `language.types:call-site-checking`: Annotated required parameters reject wrong arity and non-assignable arguments. Unannotated parameters stay permissive. Rest parameters allow extra arguments.
  test: call_too_few_required_args_errors
  test: call_arg_type_mismatch_errors
  test: call_unannotated_function_permissive
  test: call_reject_fixtures_run
- `language.types:missing-return`: An annotated non-void function whose body can fall off the end is a diagnostic.
  test: check_missing_return_errors
  test: types/reject/missing_return
- `language.types:unknown-shape-property`: Read or write of a property absent from an annotated shape is a diagnostic. Untyped objects stay dynamic.
  test: unknown_shape_prop_read_errors
  test: unknown_shape_prop_write_errors
  test: untyped_object_unknown_prop_stays_dynamic
- `language.types:excess-property`: A fresh object literal assigned to an annotated shape may not carry extra own properties.
  test: excess_prop_annotated_shape_errors
  test: excess_prop_exact_match_ok
- `language.types:annotated-non-callable`: Call or `new` of an annotated non-callable value is a diagnostic. Untyped values stay permissive at check time.
  test: call_annotated_number_errors
  test: new_annotated_number_errors
  test: call_untyped_number_ok
- `language.types:forbid-tsc-compatibility`: The Checker does not aim to compile existing TypeScript projects or match tsc flag-for-flag. The JS backend emits JavaScript, not TypeScript.
