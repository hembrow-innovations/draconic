---
id: "contract"
title: "ECMA — Contract"
kind: contract
description: "Program meaning for shipped ECMA clusters. Locked promises carry a test pointer."
status: active
domain: draconic
area: ecma
tags: [contract]
created_at: "2026-09-06"
updated_at: "2026-09-06"
---

# ECMA — Contract

Purpose: [[specs/draconic/language/purpose]]. Tests: [[specs/draconic/language/ecma/test]]. Destination ADR: [[0004-full-ecma-262-and-embed]]. Staged external bar: [[0007-test262-staged-roll-in]].

These promises are the shipped clusters, not the whole of ECMA-262.

## Behaviour

- `language.ecma:expressions`: A Program evaluates arithmetic, comparison, logical, bitwise, assignment, update, exponentiation, conditional, comma, unary-keyword, compound-assignment, and nullish/logical-assignment operators with ECMA meaning on declared targets.
  test: es/expressions/arithmetic
  test: arithmetic_runs
  test: es/expressions/comparison
  test: es/expressions/logical
  test: es/expressions/nullish_logical_assign
  test: es/legacy/implicit_global_compound
  test: implicit_global_compound_runs
- `language.ecma:statements`: A Program runs `if`/`else`, loops (`while`, `do`/`while`, `for`, `for-in`/`for-of`), `break`/`continue`, `switch`, labels, and `const` with ECMA control-flow meaning on declared targets.
  test: es/statements/if_else
  test: if_else_runs
  test: es/statements/while
  test: es/statements/switch
  test: es/statements/const
- `language.ecma:functions`: A Program declares and calls functions, nested closures, function expressions, arrows, default parameters, and rest parameters with ECMA call meaning on declared targets.
  test: es/functions/decl_return_call
  test: decl_return_call_runs
  test: es/functions/arrow
  test: es/functions/rest_params
  test: es/legacy/arguments_iterator
  test: arguments_iterator_runs
- `language.ecma:objects`: A Program builds object literals, reads and writes properties, preserves `this` on method call, constructs with `new`, and inherits prototype methods on declared targets.
  test: es/objects/object_lit_access
  test: object_lit_access_runs
  test: prototype_runs
- `language.ecma:classes`: A Program declares classes with constructors, instance methods, `extends`/`super`, and static methods on declared targets.
  test: es/classes/class_basic
  test: class_basic_runs
  test: class_extends
  test: class_static_runs
  test: class_super_access_runs
- `language.ecma:arrays`: A Program evaluates array literals, index assignment, spread in literals and calls, `for-of` over arrays, and array destructuring on declared targets.
  test: es/arrays/array_lit_access
  test: array_lit_access_runs
- `language.ecma:strings`: A Program evaluates string literals, concatenation, templates, tagged templates, Unicode escapes, and UTF-16 code-unit indexing on declared targets.
  test: es/strings/string_lit_access
  test: string_lit_access_runs
  test: template_lit_runs
  test: tagged_template_runs
  test: utf16_semantics_runs
- `language.ecma:numbers`: A Program evaluates Number literals, same-type BigInt arithmetic/comparison/bitwise/exponentiation, and global `Math`/`Number`/`NaN`/`Infinity` on declared targets.
  test: es/numbers/number_literals
  test: number_literals_runs
  test: bigint_literals_runs
  test: math_basics_runs
- `language.ecma:values`: A Program evaluates `Symbol` construction and symbol keys, plus abstract equality and `ToPrimitive` coercion on declared targets.
  test: es/values/symbol_basics
  test: symbol_basics_runs
- `language.ecma:collections`: A Program uses global `Map`/`Set` and `WeakMap`/`WeakSet` with the shipped get/set/has/size/add/delete meaning on declared targets.
  test: es/builtins/map_set
  test: map_set_runs
  test: es/builtins/weak_map_set
- `language.ecma:proxies`: A Program constructs `Proxy` and uses Reflect plus the shipped traps (get, set, has, delete, apply, construct, ownKeys, prototype, defineProperty, extensibility) on declared targets.
  test: es/proxies/proxy_basics
  test: proxy_basics_runs
- `language.ecma:modules`: A Program links static relative ESM named, default, and namespace imports, including cyclic live bindings, on declared targets.
  test: es/modules/named_export_import
  test: named_export_import_runs
- `language.ecma:async`: A Program constructs Promises, uses Promise statics (`all`/`race`/`allSettled`/`any`/`resolve`/`reject`) and `finally`, and runs `async`/`await` (including async arrows) on declared targets.
  test: es/async/promise_basics
  test: promise_basics_runs
  test: async_await_runs
- `language.ecma:generators`: A Program runs generator declarations, expressions, methods, `yield`/`yield*`, resume, `for-of`, and `.return`/`.throw` on declared targets.
  test: es/generators/basic_yield
  test: basic_yield_runs
- `language.ecma:eval`: A Program runs direct `eval`, `new Function`/`Function(...)`, and indirect eval on declared js and native targets (native via Embed).
  test: es/eval/direct_eval
  test: direct_eval_runs
  test: es/eval/new_function
  test: es/eval/indirect_eval
  test: indirect_eval_runs
  test: es/legacy/eval_let_const
  test: eval_let_const_runs
- `language.ecma:builtins`: A Program sees global `undefined`/`globalThis`, fundamental constructors, Error constructors, global functions, URI encode/decode, `JSON`, `Date`, `RegExp`, and ArrayBuffer/TypedArray/DataView basics on declared targets.
  test: es/builtins/global_basics
  test: global_basics_runs
  test: es/builtins/error_ctors
- `language.ecma:legacy-with`: A Program evaluates a `with` statement: object-environment binding, property read/write, and nested `with`.
  test: es/legacy/with_basic
  test: with_basic_runs
  test: es/legacy/with_nested
  test: es/legacy/with_var_for
  test: with_var_for_runs
  test: es/legacy/with_block_function
  test: with_block_function_runs
- `language.ecma:annex-b`: A Program evaluates shipped Annex B and related residual syntax (legacy octal, HTML comments, `__proto__`, `var` hoist, `escape`/`unescape`, and filed E18 fixtures) on declared targets.
  test: es/annex-b/escape_unescape
  test: escape_unescape_runs
  test: es/legacy/html_comments
  test: html_comments_runs
- `language.ecma:test262-staged-allowlist`: Official Test262 is a staged js-only curated allowlist plus harness, not a day-one full-suite bar. Failures stay report-only until promoted.
  test: allowlist_loads_and_has_entries
