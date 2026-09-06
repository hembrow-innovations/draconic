---
id: "rounds-587-roadmap-audit"
title: "Roadmap audit campaign"
kind: round
sitting_kind: planning
status: ready-to-resume
tags: []
created_at: "2026-09-06T11:58:21Z"
updated_at: "2026-09-07T21:00:00Z"
---
# Roadmap audit campaign

## Cursor
next: E14.04

## Ledger
- **B01**: HOLD. `cargo test -p draconic-lexer` ok. no tickets
- **B02**: HOLD. `cargo test -p draconic-parser` ok. `cargo test -p draconic-ast` ok. no tickets
- **B03**: GAP. `cargo test -p draconic-cli parse_sample_program` ok. `draconic parse` dumps AST. Tests column does not spawn CLI. [[ticket-588-b03-parse-cli-untested]]
- **B04**: HOLD. `cargo test -p draconic-check` ok. no tickets
- **B05**: HOLD. `cargo test -p draconic-check` ok. `draconic check examples/types/types.drac` exit 0. no tickets
- **B06**: HOLD. `cargo test -p draconic-ir` ok. no tickets
- **B07**: HOLD. `cargo test -p draconic-backend-js` 64 ok. `cargo test -p draconic-integration-tests --test js_backend` 7 ok. `draconic build --target js` hello-shebang. no tickets
- **B08**: HOLD. `cargo test -p draconic-backend-llvm` 305 ok. `cargo test -p draconic-runtime` 160 ok. `draconic build --target native` empty prints hello. no tickets
- **B09**: HOLD. `cargo test -p draconic-runtime` 160 ok including `gc_allocates_string_and_object_on_heap`. no tickets
- **B10**: HOLD. `cargo test -p draconic-cli --test build build_target` 3 ok. `cargo test -p draconic-integration-tests --test cli_build` 3 ok. `draconic build --target js` emits add. native prints 42. no tickets
- **E00**: HOLD. `cargo test -p draconic-conformance --test harness --lib` ok. `draconic build --target js|native` smoke/empty ok. no tickets
- **E01**: HOLD. `cargo test -p draconic-conformance --test expressions` 24 ok. `draconic build --target js` arithmetic emits operators. native owned by N08.01. no tickets
- **E01.01**: HOLD. `cargo test -p draconic-conformance --test expressions arithmetic` 2 ok. `draconic build --target js` arithmetic emits operators. native owned by N08.01.01. no tickets
- **E01.02**: HOLD. `cargo test -p draconic-conformance --test expressions comparison` 2 ok. `draconic build --target js` comparison emits operators. native owned by N08.01.02. no tickets
- **E01.03**: HOLD. `cargo test -p draconic-conformance --test expressions logical_fixture_present` ok. `logical_runs` ok. `draconic build --target js` logical emits && || !. native owned by N08.01.03. no tickets
- **E01.04**: HOLD. `cargo test -p draconic-conformance --test expressions` 24 ok. `draconic build --target js` bitwise emits operators. native owned by N08.01.04. no tickets
- **E01.04.01**: HOLD. `cargo test -p draconic-conformance --test expressions bitwise` 2 ok. `draconic build --target js` bitwise emits operators. native owned by N08.01.04.01. no tickets
- **E01.04.02**: HOLD. `cargo test -p draconic-conformance --test expressions exponentiation` 2 ok. `draconic build --target js` exponentiation emits `**` right-assoc. native owned by N08.01.04.02. no tickets
- **E01.04.03**: HOLD. `cargo test -p draconic-conformance --test expressions conditional` 2 ok. `draconic build --target js` ternary emits `? :`. native owned by N08.01.04.03. no tickets
- **E01.04.04**: HOLD. `cargo test -p draconic-conformance --test expressions assignment` 4 ok. `draconic build --target js` assignment emits `=`. native owned by N08.01.04.04. no tickets
- **E01.04.05**: HOLD. `cargo test -p draconic-conformance --test expressions update` 2 ok. `draconic build --target js` update emits ++ --. native owned by N08.01.04.05. no tickets
- **E01.04.06**: HOLD. `cargo test -p draconic-conformance --test expressions comma` 2 ok. `draconic build --target js` comma emits `,`. native owned by N08.01.04.06. no tickets
- **E01.04.07**: HOLD. `cargo test -p draconic-conformance --test expressions unary_keywords` 2 ok. `draconic build --target js` unary_keywords emits typeof void delete. native owned by N08.01.04.07. no tickets
- **E01.04.08**: HOLD. `cargo test -p draconic-conformance --test expressions compound_assignment` 2 ok. `draconic build --target js` compound_assignment emits += -= *= /= %= **= <<= >>= >>>= &= ^= |=. native owned by N08.01.04.08. no tickets
- **E01.04.09**: HOLD. `cargo test -p draconic-conformance --test expressions nullish_logical_assign` 2 ok. `draconic build --target js` nullish_logical_assign emits ?? ??= &&= ||=. native owned by N08.01.04.09. no tickets
- **E02**: HOLD. `cargo test -p draconic-conformance --test statements` 19 ok. `draconic build --target js` if_else emits if/else. native owned by N08.02. no tickets
- **E02.01**: HOLD. `cargo test -p draconic-conformance --test statements if_else` 2 ok. `draconic build --target js` if_else emits if/else. native owned by N08.02.01. no tickets
- **E02.02**: HOLD. `cargo test -p draconic-conformance --test statements while` while_fixture_present and while_runs ok. `draconic build --target js` while emits while. native owned by N08.02.02. no tickets
- **E02.03**: HOLD. `cargo test -p draconic-conformance --test statements do_while` 2 ok. `draconic build --target js` do_while emits do/while. native owned by N08.02.03. no tickets
- **E02.04**: HOLD. `cargo test -p draconic-conformance --test statements for_fixture_present` ok. `for_runs` ok. `draconic build --target js` for emits for. native owned by N08.02.04. no tickets
- **E02.05**: HOLD. `cargo test -p draconic-conformance --test statements break_continue` 2 ok. `draconic build --target js` break_continue emits break/continue. native owned by N08.02.05. no tickets
- **E02.06**: HOLD. `cargo test -p draconic-conformance --test statements switch` 2 ok. `draconic build --target js` switch emits switch/case/default/break including fall-through. native owned by N08.02.06. no tickets
- **E02.07**: HOLD. `cargo test -p draconic-conformance --test statements labeled` 2 ok. `draconic build --target js` labeled emits labeled while/for/do-while/block plus labeled break/continue. native owned by N08.02.07. no tickets
- **E02.08**: HOLD. `cargo test -p draconic-conformance --test statements for_in_of` 2 ok. `draconic build --target js` for_in_of emits for-in/for-of with let and assignment binding. native owned by N08.02.08. no tickets
- **E02.09**: HOLD. `cargo test -p draconic-conformance --test statements const` 2 ok. `draconic build --target js` const emits `const` plus for/for-of/for-in binding. native owned by N08.02.09. no tickets
- **E03**: HOLD. `cargo test -p draconic-conformance --test functions` 14 ok. `draconic build --target js` decl_return_call emits function. native owned by N08.03. no tickets
- **E03.01**: HOLD. `cargo test -p draconic-conformance --test functions decl_return_call` 2 ok. `draconic build --target js` decl_return_call emits function. native owned by N08.03.01. no tickets
- **E03.02**: HOLD. `cargo test -p draconic-conformance --test functions params_call` 2 ok. `draconic build --target js` params_call emits function params and calls. native owned by N08.03.02. no tickets
- **E03.03**: HOLD. `cargo test -p draconic-conformance --test functions nested_capture` 2 ok. `draconic build --target js` nested_capture emits nested functions capturing outer let/params. native owned by N08.03.03. no tickets
- **E03.04**: HOLD. `cargo test -p draconic-conformance --test functions function_expr` 2 ok. `draconic build --target js` function_expr emits function expressions including named and IIFE. native owned by N08.03.04. no tickets
- **E03.05**: HOLD. `cargo test -p draconic-conformance --test functions arrow` 2 ok. `draconic build --target js` arrow emits `=>` expr and block bodies. native owned by N08.03.05. no tickets
- **E03.06**: HOLD. `cargo test -p draconic-conformance --test functions default_params` 2 ok. `draconic build --target js` default_params emits `a = expr` on decl/expr/arrow. native owned by N08.03.06. no tickets
- **E03.07**: HOLD. `cargo test -p draconic-conformance --test functions rest_params` 2 ok. `draconic build --target js` rest_params emits `...args` on decl/expr/arrow. native owned by N08.03.07. no tickets
- **E04**: HOLD. `cargo test -p draconic-conformance --test objects` 14 ok. `draconic build --target js` object_lit_access emits object literals and property access. native owned by N08.04. no tickets
- **E04.01**: HOLD. `cargo test -p draconic-conformance --test objects object_lit_access` 2 ok. `draconic build --target js` object_lit_access emits `{ k: v }` and `.` / `[]` reads. native owned by N08.04.01. no tickets
- **E04.02**: HOLD. `cargo test -p draconic-conformance --test objects property_assign` 2 ok. `draconic build --target js` property_assign emits `.` / `[]` writes and read-back. native owned by N08.04.02. no tickets
- **E04.03**: HOLD. `cargo test -p draconic-conformance --test objects this_method` 2 ok. `draconic build --target js` this_method emits `obj.m()` / `obj["m"]()` and `this`. native owned by N08.04.03. no tickets
- **E04.04**: HOLD. `cargo test -p draconic-conformance --test objects new_ctor` 2 ok. `draconic build --target js` new_ctor emits `new`. native owned by N08.04.04. no tickets
- **E04.05**: HOLD. `cargo test -p draconic-conformance --test objects prototype` 2 ok. `draconic build --target js` prototype emits `.prototype` assigns, `new`, `this`. native owned by N08.04.05. no tickets
- **E04.06**: HOLD. `cargo test -p draconic-conformance --test objects object_lit_sugar` 2 ok. `draconic build --target js` object_lit_sugar emits shorthand as `a: a`, method `n()`, computed `[k]`. native owned by N08.04.06. no tickets
- **E05**: HOLD. `cargo test -p draconic-conformance --test classes` 30 ok. `draconic build --target js` class_basic emits class ctor/prototype. native owned by N08.05. no tickets
- **E05.01**: HOLD. `cargo test -p draconic-conformance --test classes class_basic` 2 ok. `draconic build --target js` class_basic emits class ctor/prototype. native owned by N08.05.01. no tickets
- **E05.02**: HOLD. `cargo test -p draconic-conformance --test classes class_extends` 2 ok. `draconic build --target js` class_extends emits heritage plus Reflect.construct super. native owned by N08.05.02. no tickets
- **E05.03**: HOLD. `cargo test -p draconic-conformance --test classes class_static` 4 ok. `draconic build --target js` class_static emits static methods on the constructor. native owned by N08.05.03. no tickets
- **E05.04**: HOLD. `cargo test -p draconic-conformance --test classes class_super_access` 2 ok. `draconic build --target js` class_super_access emits super.greet/tag/scaled. native owned by N08.05.04. no tickets
- **E06**: HOLD. `cargo test -p draconic-conformance --test arrays` 12 ok. `draconic build --target js` array_lit_access emits array lits and index access. native owned by N08.06. no tickets
- **E06.01**: HOLD. `cargo test -p draconic-conformance --test arrays array_lit_access` 2 ok. `draconic build --target js` array_lit_access emits array lits, index, and length. native owned by N08.06.01. no tickets
- **E06.02**: HOLD. `cargo test -p draconic-conformance --test arrays array_element_assign` 2 ok. `draconic build --target js` array_element_assign emits `arr[i] = v` plus read-back and length grow. native owned by N08.06.02. no tickets
- **E06.03**: HOLD. `cargo test -p draconic-conformance --test arrays array_spread` 2 ok. `draconic build --target js` array_spread emits `[...a]` / `[...a, b]` / `[a, ...b, c]`. native owned by N08.06.03. no tickets
- **E06.04**: HOLD. `cargo test -p draconic-conformance --test arrays call_spread` 2 ok. `draconic build --target js` call_spread emits `...` in call/new args. native owned by N08.06.04. no tickets
- **E06.05**: HOLD. `cargo test -p draconic-conformance --test arrays array_for_of` 2 ok. `draconic build --target js` array_for_of emits for-of with let/assign/nested. native owned by N08.06.05. no tickets
- **E06.06**: HOLD. `cargo test -p draconic-conformance --test arrays array_destructure` 2 ok. `draconic build --target js` array_destructure emits `let [a, b]`, rest, assignment. native owned by N08.06.06. no tickets
- **E07**: HOLD. `cargo test -p draconic-conformance --test strings` 10 ok. `draconic build --target js` string_lit_access emits string lits, concat, `.length`, index. native owned by N08.07. no tickets
- **E07.01**: HOLD. `cargo test -p draconic-conformance --test strings string_lit_access` 2 ok. `draconic build --target js` string_lit_access emits quotes, concat, `.length`, index, basic escapes. native owned by N08.07.01. no tickets
- **E07.02**: HOLD. `cargo test -p draconic-conformance --test strings template_lit` 2 ok. `draconic build --target js` template_lit emits untagged templates, cooked escapes, multi-interp. native owned by N08.07.02. no tickets
- **E07.03**: HOLD. `cargo test -p draconic-conformance --test strings unicode_escapes` 2 ok. `draconic build --target js` unicode_escapes cooks `\xHH` `\uXXXX` `\u{X…}` in strings and templates. native owned by N08.07.03. no tickets
- **E07.04**: HOLD. `cargo test -p draconic-conformance --test strings tagged_template` 2 ok. `draconic build --target js` tagged_template emits tag call, cooked quasis, interpolations. native owned by N08.07.04. no tickets
- **E07.05**: HOLD. `cargo test -p draconic-conformance --test strings utf16_semantics` 2 ok. `draconic build --target js` utf16_semantics emits UTF-16 units, `.length`, index, lone surrogates. native owned by N08.07.05. no tickets
- **E08**: HOLD. `cargo test -p draconic-conformance --test numbers` 12 ok. `draconic build --target js` number_literals emits decimal/hex/bin/oct/sci/separators. native owned by N08.08. no tickets
- **E08.01**: HOLD. `cargo test -p draconic-conformance --test numbers number_literals` 2 ok. `draconic build --target js` number_literals emits decimal/hex/bin/oct/sci/separators/leading-dot. native owned by N08.08.01. no tickets
- **E08.02**: HOLD. `cargo test -p draconic-conformance --test numbers bigint_literals` 2 ok. `draconic build --target js` bigint_literals emits `1n`/`0xffn`/`0b…n`/`0o…n`/`_` separators plus same-type `+ - * / %` and unary `-`. native owned by N08.08.02. no tickets
- **E08.03**: HOLD. `cargo test -p draconic-conformance --test numbers bigint_ops` 2 ok. `draconic build --target js` bigint_ops emits `<` `<=` `>` `>=` `==` `!=` `===` `!==` `&` `|` `^` `~` `<<` `>>`. native owned by N08.08.03. no tickets
- **E08.04**: HOLD. `cargo test -p draconic-conformance --test numbers bigint_pow` 2 ok. `draconic build --target js` bigint_pow emits `**` right-assoc and `**=`. native owned by N08.08.04. no tickets
- **E08.05**: HOLD. `cargo test -p draconic-conformance --test numbers math_basics` 2 ok. `draconic build --target js` math_basics emits Math.abs/floor/ceil/round/min/max/pow/sqrt/sign, Math.PI, Math.E, Math["abs"]. native owned by N08.08.05. no tickets
- **E08.06**: HOLD. `cargo test -p draconic-conformance --test numbers number_global` 2 ok. `draconic build --target js` number_global emits Number.isNaN/isFinite/isInteger/isSafeInteger, Number.NaN/POSITIVE_INFINITY/NEGATIVE_INFINITY/MAX_VALUE/MIN_VALUE/EPSILON/MAX_SAFE_INTEGER/MIN_SAFE_INTEGER, NaN, Infinity, Number["isNaN"]. native owned by N08.08.06. no tickets
- **E09**: HOLD. `cargo test -p draconic-conformance --test values` 8 ok. `draconic build --target js` symbol_basics emits Symbol/Symbol.for/Symbol.keyFor. native owned by N08.09. no tickets
- **E09.01**: HOLD. `cargo test -p draconic-conformance --test values symbol_basics` 2 ok. `draconic build --target js` symbol_basics emits Symbol/Symbol.for/Symbol.keyFor. native owned by N08.09.01. no tickets
- **E09.02**: HOLD. `cargo test -p draconic-conformance --test values symbol_property_keys` 2 ok. `draconic build --target js` symbol_property_keys emits obj[sym], computed keys, own-key read/write. native owned by N08.09.02. no tickets
- **E09.03**: HOLD. `cargo test -p draconic-conformance --test values abstract_eq_coercion` 2 ok. `draconic build --target js` abstract_eq_coercion emits == != === + unary+ if. native owned by N08.09.03. no tickets
- **E09.04**: HOLD. `cargo test -p draconic-conformance --test values to_primitive` 2 ok. `draconic build --target js` to_primitive emits valueOf/toString and + ==. native owned by N08.09.04. no tickets
- **E10**: HOLD. `cargo test -p draconic-conformance --test exceptions` 7 ok. `draconic build --target js` throw_try_catch emits try/catch/throw. native owned by N08.10. no tickets
- **E10.01**: HOLD. `cargo test -p draconic-conformance --test exceptions throw_try_catch` 2 ok. `draconic build --target js` throw_try_catch emits try/catch/throw with catch params. native owned by N08.10.01. no tickets
- **E10.02**: HOLD. `cargo test -p draconic-conformance --test exceptions try_finally` 2 ok. `draconic build --target js` try_finally emits try/catch/finally. native owned by N08.10.02. no tickets
- **E10.03**: HOLD. `cargo test -p draconic-conformance --test exceptions optional_catch` 2 ok. `draconic build --target js` optional_catch emits `catch {` with and without finally. native owned by N08.10.03. no tickets
- **E11**: HOLD. `cargo test -p draconic-conformance --test modules` 8 ok. `draconic build --target js` named_export_import emits linked bundle. native owned by N08.11. no tickets
- **E11.01**: HOLD. `cargo test -p draconic-conformance --test modules named_export_import` 2 ok. `draconic build --target js` named_export_import emits linked `export let`/`const`/`function` plus `import { x }`. native owned by N08.11.01. no tickets
- **E11.02**: HOLD. `cargo test -p draconic-conformance --test modules default_export_import` 2 ok. `draconic build --target js` default_export_import emits linked default function plus combined named; default_expr_import emits `export default` expr. native owned by N08.11.02. no tickets
- **E11.03**: HOLD. `cargo test -p draconic-conformance --test modules namespace_import` 2 ok. `draconic build --target js` namespace_import emits linked `import * as ns` plus combined default+namespace; ns props are named exports + `default`. native owned by N08.11.03. no tickets
- **E11.04**: HOLD. `cargo test -p draconic-conformance --test modules cyclic` 2 ok. `draconic build --target js` cyclic_functions and cyclic_live emit linked flattened cycle (shared live `let n`). native owned by N08.11.04. no tickets
- **E12**: HOLD. `cargo test -p draconic-conformance --test async` 18 ok. `draconic build --target js` promise_basics emits Promise/then. native prints function/42/7/2. native also owned by N06. no tickets
- **E12.01**: HOLD. `cargo test -p draconic-conformance --test async promise_basics` 2 ok. `draconic build --target js` emits new Promise and .then. native prints function/42/7/2. native also owned by N06.03. no tickets
- **E12.02**: HOLD. `cargo test -p draconic-conformance --test async promise_resolve_reject` 2 ok. `draconic build --target js` emits Promise.resolve/reject and .catch. native prints function/function/42/7/9. native also owned by N06.04. no tickets
- **E12.03**: HOLD. `cargo test -p draconic-conformance --test async promise_finally` 2 ok. `draconic build --target js` emits `.finally`. native prints function/1/1/42/7. native also owned by N06.05. no tickets
- **E12.04**: HOLD. `cargo test -p draconic-conformance --test async promise_all` ok (present+runs). `draconic build --target js` emits Promise.all. native prints function/0/2/10/20/1/2/7. native also owned by N06.06. no tickets
- **E12.05**: HOLD. `cargo test -p draconic-conformance --test async promise_race` 2 ok. `draconic build --target js` emits Promise.race. native prints function/10/1/7. native also owned by N06.07. no tickets
- **E12.06**: HOLD. `cargo test -p draconic-conformance --test async promise_all_settled` 2 ok. `draconic build --target js` emits Promise.allSettled. native prints function/0/2/fulfilled/10/rejected/7/fulfilled/1/fulfilled/2. native also owned by N06.08. no tickets
- **E12.07**: HOLD. `cargo test -p draconic-conformance --test async promise_any` 2 ok. `draconic build --target js` emits Promise.any. native prints function/10/1/1/AggregateError/2/1/AggregateError/0. native also owned by N06.09. no tickets
- **E12.08**: HOLD. `cargo test -p draconic-conformance --test async async_await` 2 ok. `draconic build --target js` emits async function decl/expr, await, throw. native prints 42/8/9/1. native also owned by N06.10. no tickets
- **E12.09**: HOLD. `cargo test -p draconic-conformance --test async async_arrow` 2 ok. `draconic build --target js` emits async arrows, await, throw. native prints 1/3/5/7. native also owned by N06.11. no tickets
- **E13**: HOLD. `cargo test -p draconic-conformance --test generators` 16 ok. `draconic build --target js` basic_yield emits function* and yield. native owned by N08.12. no tickets
- **E13.01**: HOLD. `cargo test -p draconic-conformance --test generators basic_yield` 2 ok. `draconic build --target js` basic_yield emits function* yield return and .next value/done. native owned by N08.12.01. no tickets
- **E13.02**: HOLD. `cargo test -p draconic-conformance --test generators yield_expr` 2 ok. `draconic build --target js` yield_expr emits function* yield of a+1, a*2, void 0. native owned by N08.12.02. no tickets
- **E13.03**: HOLD. `cargo test -p draconic-conformance --test generators yield_resume` 2 ok. `draconic build --target js` yield_resume emits function* yield and .next(arg). native owned by N08.12.03. no tickets
- **E13.04**: HOLD. `cargo test -p draconic-conformance --test generators yield_star` 2 ok. `draconic build --target js` yield_star emits function* and yield*. native owned by N08.12.04. no tickets
- **E13.05**: HOLD. `cargo test -p draconic-conformance --test generators generator_expr` 2 ok. `draconic build --target js` generator_expr emits function* named and IIFE. native owned by N08.12.05. no tickets
- **E13.06**: HOLD. `cargo test -p draconic-conformance --test generators generator_methods` 2 ok. `draconic build --target js` generator_methods emits object *g/*h, class *gen, static *sgen. native owned by N08.12.06. no tickets
- **E13.07**: HOLD. `cargo test -p draconic-conformance --test generators generator_for_of` 2 ok. `draconic build --target js` generator_for_of emits function* and for-of with let/assign/const, nested, continue, and early break. native owned by N08.12.07. no tickets
- **E13.08**: HOLD. `cargo test -p draconic-conformance --test generators generator_return_throw` 2 ok. `draconic build --target js` generator_return_throw emits function* yield .return .throw try/catch/finally. native owned by N08.12.08. no tickets
- **E14**: HOLD. `cargo test -p draconic-conformance --test proxies` 22 ok. `draconic build --target js` proxy_basics emits new Proxy and get trap. native owned by N08.13. no tickets
- **E14.01**: HOLD. `cargo test -p draconic-conformance --test proxies proxy_basics` 2 ok. `draconic build --target js` proxy_basics emits `new Proxy` empty-handler get and get trap. native owned by N08.13.01. no tickets
- **E14.02**: HOLD. `cargo test -p draconic-conformance --test proxies proxy_set` 2 ok. `draconic build --target js` proxy_set emits `new Proxy` empty-handler set and set trap. native owned by N08.13.02. no tickets
- **E14.03**: HOLD. `cargo test -p draconic-conformance --test proxies proxy_has` 2 ok. `draconic build --target js` proxy_has emits `in` plus `new Proxy` has trap. native owned by N08.13.03. no tickets
