---
id: "test"
title: "ECMA tests"
kind: test
description: "Which tests cover shipped ECMA Program meaning, how, and why."
status: active
domain: draconic
area: ecma
tags: [test]
created_at: "2026-09-06"
updated_at: "2026-09-07"
---

# ECMA tests

Purpose: [[specs/draconic/language/purpose]]. Contract: [[specs/draconic/language/ecma/contract]].

## Coverage

Conformance fixtures under `tests/conformance/fixtures/es/` plus harness tests in `tests/conformance/tests/`. Each cluster promise is locked by fixture id and the `*_runs` function that executes that fixture on declared targets. Mapping existing tests is the lock for this slice. This is not a claim that every ECMA-262 clause has a fixture.

Locks `language.ecma:expressions`, `language.ecma:statements`, `language.ecma:functions`, `language.ecma:objects`, `language.ecma:classes`, `language.ecma:arrays`, `language.ecma:strings`, `language.ecma:numbers`, `language.ecma:values`, `language.ecma:collections`, `language.ecma:proxies`, `language.ecma:modules`, `language.ecma:async`, `language.ecma:generators`, `language.ecma:eval`, `language.ecma:builtins`, `language.ecma:legacy-with`, `language.ecma:annex-b`, `language.ecma:test262-staged-allowlist`.

## Tests

- **tests/conformance/tests/expressions.rs** — `arithmetic_runs` (fixture `es/expressions/arithmetic`)
  - **How:** Load the arithmetic fixture and run it on every declared target; assert `ok`.
  - **Why:** Locks `language.ecma:expressions` for numeric operators. Neighbor fixtures lock comparison, logical, bitwise, assignment, and the rest of E01.
- **tests/conformance/tests/legacy.rs** — `implicit_global_compound_runs` (fixture `es/legacy/implicit_global_compound`)
  - **How:** Run unresolvable `+=` / `++` / `--` and already-created implicit-global update on declared js (no `with`).
  - **Why:** Locks `language.ecma:expressions` GetValue-first compound/update (E17.02.172) so missing identifiers throw ReferenceError and do not create `globalThis` properties.
- **tests/conformance/tests/legacy.rs** — `implicit_global_logical_runs` (fixture `es/legacy/implicit_global_logical`)
  - **How:** Run unresolvable `||=` / `&&=` / `??=` and already-created implicit-global update or short-circuit on declared js (no `with`).
  - **Why:** Locks `language.ecma:expressions` GetValue-first logical assignment (E17.02.173) so missing identifiers throw ReferenceError and do not create `globalThis` properties.
- **tests/conformance/tests/legacy.rs** — `implicit_global_unary_runs` (fixture `es/legacy/implicit_global_unary`)
  - **How:** Run unresolvable `void` / `+` / `-` / `~` / `!`, already-created implicit-global unary, and typeof/delete contrast on declared js (no `with`).
  - **Why:** Locks `language.ecma:expressions` GetValue-first unary (E17.02.174) so missing identifiers throw ReferenceError and do not create `globalThis` properties.
- **tests/conformance/tests/legacy.rs** — `implicit_global_binary_runs` (fixture `es/legacy/implicit_global_binary`)
  - **How:** Run unresolvable `+` / `==` / `&` / `**` / `<`, left-missing skip of the other operand, and already-created implicit-global binary on declared js (no `with`).
  - **Why:** Locks `language.ecma:expressions` GetValue-first binary (E17.02.175) so missing identifiers throw ReferenceError and do not create `globalThis` properties.
- **tests/conformance/tests/legacy.rs** — `implicit_global_logical_ops_runs` (fixture `es/legacy/implicit_global_logical_ops`)
  - **How:** Run unresolvable `&&` / `||` / `??`, left-missing skip, truthy/non-nullish skip of unresolvable right, and already-created implicit-global short-circuit on declared js (no `with`).
  - **Why:** Locks `language.ecma:expressions` GetValue-first logical (E17.02.176) so missing identifiers throw ReferenceError and do not create `globalThis` properties.
- **tests/conformance/tests/statements.rs** — `if_else_runs` (fixture `es/statements/if_else`)
  - **How:** Run the if/else fixture on declared targets.
  - **Why:** Locks `language.ecma:statements`. Neighbor `*_runs` functions cover loops, switch, labels, const.
- **tests/conformance/tests/functions.rs** — `decl_return_call_runs`
  - **How:** Run `es/functions/decl_return_call` on declared targets.
  - **Why:** Locks `language.ecma:functions` at declaration/return/call grain. Arrows and rest have their own `*_runs`.
- **tests/conformance/tests/legacy.rs** — `arguments_iterator_runs`
  - **How:** Run `es/legacy/arguments_iterator` on declared js (no `with`).
  - **Why:** Locks `language.ecma:functions` arguments `@@iterator` residual (E17.02.170) without wrapping the program in `with`.
- **tests/conformance/tests/objects.rs** — `object_lit_access_runs` / `prototype_runs`
  - **How:** Run object-literal and prototype fixtures.
  - **Why:** Locks `language.ecma:objects`.
- **tests/conformance/tests/classes.rs** — `class_basic_runs`
  - **How:** Run `es/classes/class_basic`; extends/static/super have sibling tests.
  - **Why:** Locks `language.ecma:classes`.
- **tests/conformance/tests/arrays.rs** — `array_lit_access_runs`
  - **How:** Run `es/arrays/array_lit_access`.
  - **Why:** Locks `language.ecma:arrays`.
- **tests/conformance/tests/strings.rs** — `string_lit_access_runs` / `template_lit_runs` / `tagged_template_runs` / `utf16_semantics_runs`
  - **How:** Run the E07 fixtures on declared targets.
  - **Why:** Locks `language.ecma:strings`.
- **tests/conformance/tests/numbers.rs** — `number_literals_runs` / `bigint_literals_runs` / `math_basics_runs`
  - **How:** Run E08 fixtures.
  - **Why:** Locks `language.ecma:numbers`.
- **tests/conformance/tests/values.rs** — `symbol_basics_runs`
  - **How:** Run `es/values/symbol_basics`.
  - **Why:** Locks `language.ecma:values`.
- **tests/conformance/tests/builtins.rs** — `map_set_runs` / `global_basics_runs`
  - **How:** Run Map/Set and global-object fixtures.
  - **Why:** Locks `language.ecma:collections` and `language.ecma:builtins`.
- **tests/conformance/tests/proxies.rs** — `proxy_basics_runs`
  - **How:** Run `es/proxies/proxy_basics`; trap fixtures are siblings.
  - **Why:** Locks `language.ecma:proxies`.
- **tests/conformance/tests/modules.rs** — `named_export_import_runs`
  - **How:** Run static relative named import/export; default/namespace/cycle are siblings.
  - **Why:** Locks `language.ecma:modules`.
- **tests/conformance/tests/async.rs** — `promise_basics_runs` / `async_await_runs`
  - **How:** Run Promise and async/await fixtures on js and native.
  - **Why:** Locks `language.ecma:async`.
- **tests/conformance/tests/generators.rs** — `basic_yield_runs`
  - **How:** Run `es/generators/basic_yield`.
  - **Why:** Locks `language.ecma:generators`.
- **tests/conformance/tests/eval.rs** — `direct_eval_runs` / `indirect_eval_runs`
  - **How:** Run eval fixtures on js and native (Embed on native).
  - **Why:** Locks `language.ecma:eval`.
- **tests/conformance/tests/legacy.rs** — `eval_let_const_runs`
  - **How:** Run `es/legacy/eval_let_const` on declared js (no `with`).
  - **Why:** Locks `language.ecma:eval` lexical eval instantiation (E17.02.171) so `let`/`const` do not inject into the caller VariableEnvironment.
- **tests/conformance/tests/legacy.rs** — `with_basic_runs`
  - **How:** Run `es/legacy/with_basic` on declared targets.
  - **Why:** Locks `language.ecma:legacy-with`. Filed E17.02 children have more `with_*` fixtures; untracked remainder is out of scope.
- **tests/conformance/tests/legacy.rs** — `with_var_for_runs`
  - **How:** Run `es/legacy/with_var_for` on declared js, including `for (const …)` for-in and for-of heads inside `with`, and getter `this` on for-in/for-of right idents and Annex B initializer RHS.
  - **Why:** Locks `language.ecma:legacy-with` so lexical `const` for-heads do not HasBinding-assign onto the with object, and accessor get `this` is the with object (E17.02.129).
- **tests/conformance/tests/legacy.rs** — `with_block_function_runs`
  - **How:** Run `es/legacy/with_block_function` on declared js, including a skipped `{ function k(){…} }` inside `with` (not around it).
  - **Why:** Locks `language.ecma:legacy-with` so a skipped block-level function is not assigned through the object environment while `with` is on the stack (E17.02.127).
- **tests/conformance/tests/legacy.rs** — `with_string_trim_left_right_runs`
  - **How:** Run `es/legacy/with_string_trim_left_right` on declared js, including args-hit-object and args-miss-uses-outer for `trimLeft` / `trimRight`.
  - **Why:** Locks `language.ecma:legacy-with` so argument IdentifierReferences to those methods HasBinding on the with object, else outer (E17.02.120). Unresolvable `e1702120_arg` is not this lock.
- **tests/conformance/tests/legacy.rs** — `with_object_accessor_legacy_runs`
  - **How:** Run `es/legacy/with_object_accessor_legacy` on declared js, including args-hit-object and args-miss-uses-outer for `__defineGetter__` / `__defineSetter__` / `__lookupGetter__` / `__lookupSetter__`.
  - **Why:** Locks `language.ecma:legacy-with` so argument IdentifierReferences to those methods HasBinding on the with object, else outer (E17.02.121). Unresolvable `e1702121_arg` is not this lock.
- **tests/conformance/tests/legacy.rs** — `html_comments_runs`
  - **How:** Run `es/legacy/html_comments` on declared js (no `with`).
  - **Why:** Locks `language.ecma:annex-b` HTML-like comments (E17.02.169) without wrapping the program in `with`.
- **tests/conformance/tests/annex_b.rs** — `escape_unescape_runs`
  - **How:** Run `es/annex-b/escape_unescape`.
  - **Why:** Locks `language.ecma:annex-b` at cluster grain.
- **tests/test262/src/lib.rs** — `allowlist_loads_and_has_entries`
  - **How:** Load the curated Test262 allowlist and assert it has a large js-only entry set.
  - **Why:** Locks `language.ecma:test262-staged-allowlist`. Does not lock the full Test262 suite.

## Gaps

- No test title locks “literally all of ECMA-262.” Destination is [[0004-full-ecma-262-and-embed]]; shipped meaning is the clusters above.
- Roadmap **E17.02** untracked remainder has no promise and no lock (purpose fence).
- Exception syntax is mapped in [[specs/draconic/language/exceptions/test]], not here.
