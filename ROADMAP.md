# Draconic Roadmap

Source of truth for completeness, together with the test suite.  
**Status**: `todo` | `in_progress` | `done` | `blocked`

A item is `done` only when its tests are green on every applicable target (`js`, `native`, or both).

## Legend

- **Targets**: `js` | `native` | `both` | `compiler` (toolchain-only, no program emit)
- **Tests**: path(s) that must pass

---

## B — Bootstrap (spine before full Conformance velocity)

| ID | Status | Targets | Item | Tests |
|----|--------|---------|------|-------|
| B01 | done | compiler | Lexer: scan source into tokens (identifiers, keywords, punctuators, literals, EOF) | `crates/draconic-lexer` |
| B02 | done | compiler | Parser + AST: parse a minimal Program; AST dump stable for snapshots | `crates/draconic-parser`, `crates/draconic-ast` |
| B03 | done | compiler | CLI: `draconic parse <file>` prints AST dump | `crates/draconic-cli` |
| B04 | done | compiler | Binder: scopes and symbol resolution for minimal Program | `crates/draconic-check` |
| B05 | done | compiler | Checker: TypeScript-inspired types for minimal Program | `crates/draconic-check` |
| B06 | done | compiler | Shared IR: lower minimal typed Program to IR | `crates/draconic-ir` |
| B07 | done | js | JS backend: emit runnable JS for minimal Program | `crates/draconic-backend-js`, `tests/integration` |
| B08 | done | native | LLVM backend stub + Runtime hello: native binary prints | `crates/draconic-backend-llvm`, `crates/draconic-runtime` |
| B09 | done | native | GC hello: allocate string/object on native heap | `crates/draconic-runtime` |
| B10 | done | both | CLI: `draconic build --target js\|native` end-to-end | `crates/draconic-cli`, `tests/integration` |

---

## E — ECMA-262 Conformance (grow via Loop; cluster by area)

Each cluster expands into finer rows as the Loop reaches it. Until then the cluster is the unit.

| ID | Status | Targets | Item | Tests |
|----|--------|---------|------|-------|
| E00 | done | both | Conformance harness: load fixtures, run on js + native runners | `tests/conformance` |
| E01 | done | both | Expressions & operators (ECMA-262 §12–13 core) | `tests/conformance/es/expressions` |
| E01.01 | done | both | Numeric arithmetic: `+` `-` `*` `/` `%`, unary `+`/`-`, grouping/precedence | `tests/conformance` fixtures `es/expressions` |
| E01.02 | done | both | Comparison & equality: `<` `<=` `>` `>=` `==` `!=` `===` `!==` | `tests/conformance` fixtures `es/expressions` |
| E01.03 | done | both | Logical: `&&` `\|\|` `!` | `tests/conformance` fixtures `es/expressions` |
| E01.04 | done | both | Remaining §12–13 (bitwise, assignment, conditional, update, `**`, comma, …) | `tests/conformance` fixtures `es/expressions` |
| E01.04.01 | done | both | Bitwise: `&` `\|` `^` `~` `<<` `>>` `>>>` | `tests/conformance` fixtures `es/expressions` |
| E01.04.02 | done | both | Exponentiation: `**` (right-associative) | `tests/conformance` fixtures `es/expressions` |
| E01.04.03 | done | both | Conditional (ternary): `cond ? a : b` | `tests/conformance` fixtures `es/expressions` |
| E01.04.04 | done | both | Assignment: `=` (simple, right-associative) | `tests/conformance` fixtures `es/expressions` |
| E01.04.05 | done | both | Update: prefix/postfix `++` `--` | `tests/conformance` fixtures `es/expressions` |
| E01.04.06 | done | both | Comma operator: `,` (left-to-right, yields RHS) | `tests/conformance` fixtures `es/expressions` |
| E01.04.07 | done | both | Unary keywords: `typeof` `void` `delete` | `tests/conformance` fixtures `es/expressions` |
| E01.04.08 | done | both | Compound assignment: `+=` `-=` `*=` `/=` `%=` `**=` `<<=` `>>=` `>>>=` `&=` `^=` `\|=` | `tests/conformance` fixtures `es/expressions` |
| E01.04.09 | done | both | Nullish coalescing `??` and logical assignment: `&&=` `\|\|=` `??=` | `tests/conformance` fixtures `es/expressions` |
| E02 | done | both | Statements & control flow (§14) | `tests/conformance/es/statements` |
| E02.01 | done | both | `if` / `else` (incl. block bodies) | `tests/conformance` fixtures `es/statements` |
| E02.02 | done | both | `while` loops (incl. block bodies) | `tests/conformance` fixtures `es/statements` |
| E02.03 | done | both | `do` / `while` loops (incl. block bodies) | `tests/conformance` fixtures `es/statements` |
| E02.04 | done | both | `for` loops: `for (init; test; update)` (incl. `let` init, omitted clauses, block bodies) | `tests/conformance` fixtures `es/statements` |
| E02.05 | done | both | `break` / `continue` (unlabeled, in loops) | `tests/conformance` fixtures `es/statements` |
| E02.06 | done | both | `switch` / `case` / `default` (incl. fall-through, `break`) | `tests/conformance` fixtures `es/statements` |
| E02.07 | done | both | Labeled statements + labeled `break` / `continue` | `tests/conformance` fixtures `es/statements` |
| E02.08 | done | both | `for-in` / `for-of` loops (incl. `let` binding, block bodies; iterate strings) | `tests/conformance` fixtures `es/statements` |
| E02.09 | done | both | `const` declarations (required initializer; no reassignment; `for`/`for-of`/`for-in` binding) | `tests/conformance` fixtures `es/statements` |
| E03 | done | both | Functions, closures, arguments, arrows (§15) | `tests/conformance/es/functions` |
| E03.01 | done | both | Function declaration + `return` + call (no params) | `tests/conformance` fixtures `es/functions` |
| E03.02 | done | both | Function parameters + call with arguments | `tests/conformance` fixtures `es/functions` |
| E03.03 | done | both | Nested function declarations + free-variable capture (outer `let`/params) | `tests/conformance` fixtures `es/functions` |
| E03.04 | done | both | Function expressions: `function (params) { … }` as values (incl. named, IIFE) | `tests/conformance` fixtures `es/functions` |
| E03.05 | done | both | Arrow functions: `(params) => expr` and `(params) => { … }` (simple ident params; no `this`) | `tests/conformance` fixtures `es/functions` |
| E03.06 | done | both | Default parameters: `function f(a = expr)` / `(a = expr) => …` (missing/`undefined` → default) | `tests/conformance` fixtures `es/functions` |
| E03.07 | done | both | Rest parameters: `function f(...args)` / `(...args) => …` (last param; binds array of remaining args) | `tests/conformance` fixtures `es/functions` |
| E04 | done | both | Objects, prototypes, `this`, property access (§10, §20) | `tests/conformance/es/objects` |
| E04.01 | done | both | Object literals `{ k: v }` + property access `obj.k` / `obj["k"]` (read; string keys) | `tests/conformance` fixtures `es/objects` |
| E04.02 | done | both | Property assignment: `obj.k = v` / `obj["k"] = v` (simple `=`; read-back) | `tests/conformance` fixtures `es/objects` |
| E04.03 | done | both | `this` + method call: `obj.m()` / `obj["m"]()` preserves `this`; `function` methods as values | `tests/conformance` fixtures `es/objects` |
| E04.04 | done | both | `new` constructor: `new F(args)` with `function` ctor setting `this` props; returns instance | `tests/conformance` fixtures `es/objects` |
| E04.05 | done | both | Prototypes: `F.prototype.m = function…`; instances inherit methods; `this` in prototype methods | `tests/conformance` fixtures `es/objects` |
| E04.06 | done | both | Object literal sugar: property shorthand `{ a }`, method shorthand `{ m() {…} }`, computed keys `{ [expr]: v }` | `tests/conformance` fixtures `es/objects` |
| E05 | done | both | Classes (§15.7) | `tests/conformance/es/classes` |
| E05.01 | done | both | Class declaration: `class C { constructor(…) {…} m(…) {…} }`; `new C(args)`; instance methods + `this` | `tests/conformance` fixtures `es/classes` |
| E05.02 | done | both | Class heritage: `extends Parent`; `super(…)` in constructor; inherit parent prototype methods | `tests/conformance` fixtures `es/classes` |
| E05.03 | done | both | Static methods: `static m(…) {…}`; call as `C.m(…)` / `new C().constructor.m` not required; no `this` instance binding | `tests/conformance` fixtures `es/classes` |
| E05.04 | done | both | `super` property access: `super.m(…)` / `super.prop` in methods (parent prototype, correct `this`) | `tests/conformance` fixtures `es/classes` |
| E06 | done | both | Arrays, iterators, spread/rest | `tests/conformance/es/arrays` |
| E06.01 | done | both | Array literals `[a, b]` + index access `arr[i]` / `arr.length` (read) | `tests/conformance` fixtures `es/arrays` |
| E06.02 | done | both | Array element assignment: `arr[i] = v` (simple `=`; read-back; length may grow) | `tests/conformance` fixtures `es/arrays` |
| E06.03 | done | both | Spread in array literals: `[...a]` / `[...a, b]` / `[a, ...b, c]` (iterable copy/concat) | `tests/conformance` fixtures `es/arrays` |
| E06.04 | done | both | Spread in call/new args: `f(...a)` / `f(x, ...a, y)` / `new F(...a)` (iterable expand into arguments) | `tests/conformance` fixtures `es/arrays` |
| E06.05 | done | both | `for-of` over arrays: iterate elements; `let`/assign binding; block bodies; nested arrays | `tests/conformance` fixtures `es/arrays` |
| E06.06 | done | both | Array destructuring: `let [a, b] = arr` / `let [a, ...rest] = arr` / assignment `[a, b] = arr` (holes, defaults deferred) | `tests/conformance` fixtures `es/arrays` |
| E07 | done | both | Strings, template literals, UTF-16 semantics | `tests/conformance/es/strings` |
| E07.01 | done | both | String literals (`"`/`'`) + concat `+` + `.length` + index `s[i]` (read; basic escapes `\n` `\r` `\t` `\\` `\'` `\"` `\0`) | `tests/conformance` fixtures `es/strings` |
| E07.02 | done | both | Template literals: `` `…` `` / `` `a${x}b` `` (untagged; cooked escapes; multi-interp) | `tests/conformance` fixtures `es/strings` |
| E07.03 | done | both | Unicode escapes: `\xHH`, `\uXXXX`, `\u{X…}` in string/template literals (cooked; well-formed scalar values) | `tests/conformance` fixtures `es/strings` |
| E07.04 | done | both | Tagged templates: `tag\`…\`` / `tag\`a${x}b\`` (tag call; cooked quasis + interpolations as args) | `tests/conformance` fixtures `es/strings` |
| E07.05 | done | both | UTF-16 code-unit semantics: `.length`/index by code unit; lone surrogates via `\uXXXX`; surrogate-pair escapes equal code-point | `tests/conformance` fixtures `es/strings` |
| E08 | done | both | Numbers, BigInt, Math, bitwise | `tests/conformance/es/numbers` |
| E08.01 | done | both | Number literals: decimal floats, scientific `e`/`E`, hex `0x`, binary `0b`, octal `0o`, numeric separators `_`, leading-dot (`.5`) | `tests/conformance` fixtures `es/numbers` |
| E08.02 | done | both | BigInt integer literals (`1n`, `0xffn`, `0b…n`, `0o…n`, `_` separators) + same-type `+` `-` `*` `/` `%` and unary `-` | `tests/conformance` fixtures `es/numbers` |
| E08.03 | done | both | BigInt comparison & bitwise: `<` `<=` `>` `>=` `==` `!=` `===` `!==` `&` `\|` `^` `~` `<<` `>>` (no `>>>`) | `tests/conformance` fixtures `es/numbers` |
| E08.04 | done | both | BigInt exponentiation: `**` (right-associative) and `**=` (same-type BigInt only) | `tests/conformance` fixtures `es/numbers` |
| E08.05 | done | both | Global `Math`: constants (`E`, `PI`) + methods (`abs`, `floor`, `ceil`, `round`, `min`, `max`, `pow`, `sqrt`, `sign`) via `.` / `[]` and calls | `tests/conformance` fixtures `es/numbers` |
| E08.06 | done | both | Global `NaN` / `Infinity` + `Number`: constants (`NaN`, `POSITIVE_INFINITY`, `NEGATIVE_INFINITY`, `MAX_VALUE`, `MIN_VALUE`, `EPSILON`, `MAX_SAFE_INTEGER`, `MIN_SAFE_INTEGER`) + static methods (`isNaN`, `isFinite`, `isInteger`, `isSafeInteger`) via `.` / `[]` and calls | `tests/conformance` fixtures `es/numbers` |
| E09 | done | both | Symbols, equality, coercion rules | `tests/conformance/es/values` |
| E09.01 | done | both | Symbol constructor basics: `Symbol()` / `Symbol(desc)`, `typeof` `"symbol"`, uniqueness; `Symbol.for` / `Symbol.keyFor` | `tests/conformance` fixtures `es/values` |
| E09.02 | done | both | Symbol property keys: `obj[sym]`, computed keys in literals; own-key read/write (no string collision) | `tests/conformance` fixtures `es/values` |
| E09.03 | done | both | Abstract equality & coercion: `==`/`!=` mixed types; `ToNumber`/`ToString`/`ToBoolean` via `+`/`==`/`if` | `tests/conformance` fixtures `es/values` |
| E09.04 | done | both | `ToPrimitive`: `valueOf` / `toString` hooks in `+` and `==` | `tests/conformance` fixtures `es/values` |
| E10 | done | both | Exceptions: try/catch/finally/throw | `tests/conformance/es/exceptions` |
| E10.01 | done | both | `throw` expression + bare `try`/`catch` (bind catch param; no finally) | `tests/conformance` fixtures `es/exceptions` |
| E10.02 | done | both | `finally`: `try`/`catch`/`finally` and `try`/`finally` (always runs; completion after finally) | `tests/conformance` fixtures `es/exceptions` |
| E10.03 | done | both | Optional catch binding: `catch { … }` (no param; with/without `finally`) | `tests/conformance` fixtures `es/exceptions` |
| E11 | done | both | Modules (ESM): import/export, cyclic | `tests/conformance/es/modules` |
| E11.01 | done | both | Named export + import: `export let`/`const`/`function`, `import { x } from "./mod"` (static relative; no default/star/cycles) | `tests/conformance` fixtures `es/modules` |
| E11.02 | done | both | Default export + import: `export default …` / `export default function…`, `import d from "./mod"` / `import d, { x } from "./mod"` (static relative; no star/cycles) | `tests/conformance` fixtures `es/modules` |
| E11.03 | done | both | Namespace import: `import * as ns from "./mod"` / `import d, * as ns from "./mod"` (static relative; ns props are named exports + `default` when present; no cycles) | `tests/conformance` fixtures `es/modules` |
| E11.04 | done | both | Cyclic modules: mutual static relative imports; named exports as functions + live `let` bindings (no namespace/`export *` in cycle fixtures) | `tests/conformance` fixtures `es/modules` |
| E12 | done | both | Promises, job queue, async/await | `tests/conformance/es/async` |
| E12.01 | done | both | Promise constructor basics: `new Promise(executor)`, sync `resolve`/`reject`, one-hop `.then`, `typeof Promise` | `tests/conformance` fixtures `es/async` |
| E12.02 | done | both | Promise statics + catch: `Promise.resolve` / `Promise.reject`, `.catch`; sync settle values; typeof of statics | `tests/conformance` fixtures `es/async` |
| E12.03 | done | both | `Promise.prototype.finally`: fulfill + reject paths; value/reason pass-through; `typeof p.finally` | `tests/conformance` fixtures `es/async` |
| E12.04 | done | both | `Promise.all`: iterable of promises/values; fulfill with array; reject on first rejection; empty → `[]` | `tests/conformance` fixtures `es/async` |
| E12.05 | done | both | `Promise.race`: iterable of promises/values; settle with first fulfillment or rejection | `tests/conformance` fixtures `es/async` |
| E12.06 | done | both | `Promise.allSettled`: iterable of promises/values; fulfill with `{status,value\|reason}[]`; empty → `[]` | `tests/conformance` fixtures `es/async` |
| E12.07 | done | both | `Promise.any`: iterable of promises/values; fulfill with first fulfillment; reject `AggregateError` if all reject; empty → reject | `tests/conformance` fixtures `es/async` |
| E12.08 | done | both | `async function` + `await`: declaration/expression, `await` expr; returns Promise; sync throw → reject | `tests/conformance` fixtures `es/async` |
| E12.09 | done | both | Arrow functions: `async (params) => expr` / `async (params) => { … }` (simple ident params; `await` in body; returns Promise) | `tests/conformance` fixtures `es/async` |
| E13 | done | both | Generators, `yield` | `tests/conformance/es/generators` |
| E13.01 | done | both | Generator function declaration: `function* g() { yield expr; return expr; }`; call returns iterator; `.next()` → `{value, done}` | `tests/conformance` fixtures `es/generators` |
| E13.02 | done | both | Yield expression RHS: AssignmentExpression-level (not unary); bare `yield` → undefined; multiple sequential yields; generator params | `tests/conformance` fixtures `es/generators` |
| E13.03 | done | both | Yield resume: `.next(arg)` becomes the value of the paused `yield` expr; first `.next()` arg ignored; bare/`yield expr` both resume | `tests/conformance` fixtures `es/generators` |
| E13.04 | done | both | `yield*`: delegate to iterable/iterator (`yield* expr`); flatten nested generators; completion value of inner | `tests/conformance` fixtures `es/generators` |
| E13.05 | done | both | Generator function expressions: `function* (params) { … }` as values (incl. named, IIFE); call returns iterator | `tests/conformance` fixtures `es/generators` |
| E13.06 | done | both | Generator methods: object `{ *m() {…} }` / class `*m()` / `static *m()`; call returns iterator; `yield` in method body | `tests/conformance` fixtures `es/generators` |
| E13.07 | done | both | `for-of` over generators: iterate yielded values; `let`/assign binding; block bodies; early `break` | `tests/conformance` fixtures `es/generators` |
| E13.08 | done | both | Generator `.return(value)` / `.throw(exception)`: close with value; inject exception at paused `yield`; try/catch in body | `tests/conformance` fixtures `es/generators` |
| E14 | done | both | Proxies, Reflect, exotic objects | `tests/conformance/es/proxies` |
| E14.01 | done | both | Proxy constructor basics: `new Proxy(target, handler)`; empty-handler pass-through get; `get` trap; `typeof Proxy` | `tests/conformance` fixtures `es/proxies` |
| E14.02 | done | both | Proxy `set` trap: empty-handler pass-through set; `set` trap intercept write; read-back | `tests/conformance` fixtures `es/proxies` |
| E14.03 | done | both | Proxy `has` trap + `in`: empty-handler pass-through `"k" in obj`; `has` trap intercept; plain object `in` | `tests/conformance` fixtures `es/proxies` |
| E14.04 | done | both | Proxy `deleteProperty` trap + `delete`: empty-handler pass-through `delete obj.k`; trap intercept; plain object `delete` | `tests/conformance` fixtures `es/proxies` |
| E14.05 | done | both | Proxy `apply` trap: empty-handler pass-through call; `apply` trap intercept call; callable target | `tests/conformance` fixtures `es/proxies` |
| E14.06 | done | both | Proxy `construct` trap: empty-handler pass-through `new`; `construct` trap intercept `new`; constructable target | `tests/conformance` fixtures `es/proxies` |
| E14.07 | done | both | Reflect basics: `typeof Reflect`; `Reflect.get`/`set`/`has`/`deleteProperty`/`apply`/`construct` on plain objects + Proxy targets | `tests/conformance` fixtures `es/proxies` |
| E14.08 | done | both | Proxy `ownKeys` trap + `Reflect.ownKeys`: empty-handler pass-through; trap intercept; plain object keys | `tests/conformance` fixtures `es/proxies` |
| E14.09 | done | both | Proxy `getPrototypeOf`/`setPrototypeOf` traps + `Reflect.getPrototypeOf`/`setPrototypeOf`: empty-handler pass-through; trap intercept; plain object | `tests/conformance` fixtures `es/proxies` |
| E14.10 | done | both | Proxy `defineProperty`/`getOwnPropertyDescriptor` traps + `Reflect.defineProperty`/`getOwnPropertyDescriptor`: empty-handler pass-through; trap intercept; plain object data descriptors | `tests/conformance` fixtures `es/proxies` |
| E14.11 | done | both | Proxy `isExtensible`/`preventExtensions` traps + `Reflect.isExtensible`/`preventExtensions`: empty-handler pass-through; trap intercept; plain object | `tests/conformance` fixtures `es/proxies` |
| E15 | done | both | Realms, globals, built-ins surface | `tests/conformance/es/builtins` |
| E15.01 | done | both | Global object basics: `undefined`, `globalThis`, fundamental constructors `Object`/`Function`/`Array`/`String`/`Boolean` (`typeof`) | `tests/conformance` fixtures `es/builtins` |
| E15.02 | done | both | Error constructors: `Error` / `TypeError` / `RangeError` / `ReferenceError` / `SyntaxError` / `URIError` / `EvalError` / `AggregateError` (`typeof`, `globalThis` identity, `new …(msg)`, `.name`/`.message`) | `tests/conformance` fixtures `es/builtins` |
| E15.03 | done | both | Global functions: `parseInt` / `parseFloat` / `isNaN` / `isFinite` (`typeof`, `globalThis` identity, basic call behavior) | `tests/conformance` fixtures `es/builtins` |
| E15.04 | done | both | URI encode/decode: `encodeURI` / `decodeURI` / `encodeURIComponent` / `decodeURIComponent` (`typeof`, `globalThis` identity, basic call behavior) | `tests/conformance` fixtures `es/builtins` |
| E15.05 | done | both | Global `JSON`: `typeof JSON`, `globalThis` identity, `JSON.parse` / `JSON.stringify` basics | `tests/conformance` fixtures `es/builtins` |
| E15.06 | done | both | Global `Date`: `typeof Date`, `globalThis` identity, `Date.now`, `new Date(ms)` / `.getTime()` / `.valueOf()` basics | `tests/conformance` fixtures `es/builtins` |
| E15.07 | done | both | Global `RegExp`: `typeof RegExp`, `globalThis` identity, `new RegExp(pattern)` / `new RegExp(pattern, flags)`, `.test` / `.exec` / `.source` / `.flags` basics | `tests/conformance` fixtures `es/builtins` |
| E15.08 | done | both | Global `Map` / `Set`: `typeof`, `globalThis` identity, `new Map` / `new Set`, `.set`/`.get`/`.has`/`.size` and `.add`/`.has`/`.size` basics | `tests/conformance` fixtures `es/builtins` |
| E15.09 | done | both | Global `WeakMap` / `WeakSet`: `typeof`, `globalThis` identity, `new WeakMap` / `new WeakSet`, `.set`/`.get`/`.has`/`.delete` and `.add`/`.has`/`.delete` (object keys only) | `tests/conformance` fixtures `es/builtins` |
| E15.10 | done | both | Global `ArrayBuffer` / `DataView` / TypedArrays: `typeof`, `globalThis` identity, `new ArrayBuffer(n)` / `.byteLength`, `new Uint8Array`/`Int32Array`/`Float64Array`, `.length`/index read-write, `new DataView(buf)` / `.getUint8`/`.setUint8` | `tests/conformance` fixtures `es/builtins` |
| E16 | done | both | `eval`, `new Function`, indirect eval | `tests/conformance/es/eval` |
| E16.01 | done | both | Direct `eval`: `typeof eval`, `globalThis` identity, `eval(string)` expression/statement basics | `tests/conformance` fixtures `es/eval` |
| E16.02 | done | both | `new Function` / `Function(...)`: construct from strings; call returns function; simple body/params | `tests/conformance` fixtures `es/eval` |
| E16.03 | done | both | Indirect eval: `(0, eval)(s)` / `globalThis.eval(s)` (global scope; not caller lexical) | `tests/conformance` fixtures `es/eval` |
| E17 | done | both | `with`, non-strict legacy where required by 262 | `tests/conformance/es/legacy` |
| E18 | done | both | Remaining Annex B / full 262 gaps (track explicitly, do not drop) | `tests/conformance/es/annex-b` |
| E18.43 | done | both | Async generators: `async function* g(){ yield expr; }` / `async function* (params){…}` / `{ async *m(){…} }` / class `async *m()`; call returns async iterator; `.next()` → Promise of `{value,done}`; `for await` over async gen | `tests/conformance` fixtures `es/annex-b` |
| E18.42 | done | both | `for await…of`: `for await (let x of asyncIterable)` / assign binding; async iter protocol; only in async functions | `tests/conformance` fixtures `es/annex-b` |
| E18.41 | done | both | Class static initialization blocks: `static { … }` (runs at class eval; access private statics; multiple blocks in order) | `tests/conformance` fixtures `es/annex-b` |
| E18.40 | done | both | Private brand check: `#x in obj` (true iff obj has the private field/method/accessor; false for null/undefined/unrelated) | `tests/conformance` fixtures `es/annex-b` |
| E18.39 | done | both | Private accessors: `get #x()` / `set #x(v)` / `static get`/`set #x`; read/write via `this.#x` / `C.#x` (not public; not on prototype) | `tests/conformance` fixtures `es/annex-b` |
| E18.38 | done | both | Static private methods: `class C { static #m(…){…} }` / call `C.#m(…)` / `this.#m(…)` in static (not on instances; not public) | `tests/conformance` fixtures `es/annex-b` |
| E18.37 | done | both | Private instance methods: `class C { #m(…){…} }` / call `this.#m(…)` (not on prototype; not public) | `tests/conformance` fixtures `es/annex-b` |
| E18.35 | done | both | Private instance fields: `class C { #x = expr; #y; m(){ this.#x } }` / assign `this.#x = v` (simple `=`; not public props) | `tests/conformance` fixtures `es/annex-b` |
| E18.36 | done | both | Static private fields: `class C { static #x = expr; static #y; m(){ C.#x } }` / assign `C.#x = v` (class-side; not on instances) | `tests/conformance` fixtures `es/annex-b` |
| E18.34 | done | both | Async methods: object `{ async m(){…} }` / class `async m()` / `static async m()`; `await` in body; returns Promise | `tests/conformance` fixtures `es/annex-b` |
| E18.33 | done | both | Class expressions: `let C = class {…}` / `let C = class Name {…}` (methods, `extends`, fields; `new C`) | `tests/conformance` fixtures `es/annex-b` |
| E18.32 | done | both | `export class C` / `export default class C`: named class export and default; consumer `import { C }` / `import C from` | `tests/conformance` fixtures `es/annex-b` |
| E18.31 | done | both | `export * as ns from`: namespace re-export from static relative module; consumer `import { ns }` / `import * as m` sees `ns` as module namespace object | `tests/conformance` fixtures `es/annex-b` |
| E18.30 | done | both | `export { x } from` / `export { x as y } from` / `export { default as d } from`: named re-export from static relative module | `tests/conformance` fixtures `es/annex-b` |
| E18.29 | done | both | `export * from`: re-export all named exports from a static relative module; `import { x } from` consumer sees re-exported names | `tests/conformance` fixtures `es/annex-b` |
| E18.28 | done | both | Object spread in literals: `{...a}` / `{...a, b}` / `{a, ...b, c}` (copy enumerable own string keys; later props overwrite) | `tests/conformance` fixtures `es/annex-b` |
| E18.27 | done | both | `new.target`: meta-property in `function`/`constructor`; `undefined` in non-`new` call; subclass points at active construct | `tests/conformance` fixtures `es/annex-b` |
| E18.26 | done | both | Class public fields: `class C { x = expr; static y = expr; }` (simple ident; instance after `super`; optional init → undefined) | `tests/conformance` fixtures `es/annex-b` |
| E18.25 | done | both | Parameter destructuring: `function f({a})` / `function f([a])` / arrows; rename, nested, rest, defaults | `tests/conformance` fixtures `es/annex-b` |
| E18.24 | done | both | `arguments` object: `arguments[i]` / `arguments.length` in non-arrow `function` (call arity; not arrow) | `tests/conformance` fixtures `es/annex-b` |
| E18.23 | done | both | Optional chaining: `a?.b` / `a?.[expr]` / `a?.(args)` (short-circuit on `null`/`undefined`; chainable) | `tests/conformance` fixtures `es/annex-b` |
| E18.22 | done | both | Accessor properties: object/class `get name(){}` / `set name(v){}` (incl. `static get`/`set`; read/write via property access) | `tests/conformance` fixtures `es/annex-b` |
| E18.21 | done | both | `instanceof`: `obj instanceof Ctor` (function/class constructors; prototype chain) | `tests/conformance` fixtures `es/annex-b` |
| E18.20 | done | both | Destructuring defaults: `let [a = expr] = arr` / `let {a = expr} = obj` / `{a: b = expr}` / assignment patterns with defaults | `tests/conformance` fixtures `es/annex-b` |
| E18.19 | done | both | Object destructuring: `let {a, b} = obj` / `let {a, ...rest} = obj` / assignment `{a, b} = obj` (rename, nested; defaults deferred) | `tests/conformance` fixtures `es/annex-b` |
| E18.18 | done | both | Regular expression literals: `/pattern/` / `/pattern/flags`; `typeof` `"object"`; `.source`/`.flags`/`.test`/`.exec` parity with `new RegExp` | `tests/conformance` fixtures `es/annex-b` |
| E18.17 | done | both | VariableStatements in Catch (Annex B.3.4): `catch (e) { var e … }` allowed; var hoists to VariableEnvironment; initializer assigns catch binding | `tests/conformance` fixtures `es/annex-b` |
| E18.16 | done | both | RegExp constructor Annex B statics (B.2.5): `$1`–`$9`, `input`/`$_`, `lastMatch`/`$&`, `lastParen`/`$+`, `leftContext`/`$\``, `rightContext`/`$'` after match/exec | `tests/conformance` fixtures `es/annex-b` |
| E18.15 | done | both | `var` in `for` heads: `for (var i=…;…;…)`, `for (var k in/of …)`, Annex B.3.5 `for (var k = init in obj)` | `tests/conformance` fixtures `es/annex-b` |
| E18.14 | done | both | `var` declarations: `var x` / `var x = expr` (function-scoped hoist; redeclaration; no TDZ; simple ident) | `tests/conformance` fixtures `es/annex-b` |
| E18.13 | done | both | Block-level function declarations (Annex B.3.2): `{ function f(){…} }` (non-strict); name block-local + enclosing var-like binding; assigned when block runs | `tests/conformance` fixtures `es/annex-b` |
| E18.12 | done | both | FunctionDeclarations in `if` (Annex B.3.4): `if (c) function f(){…}` / `else function f(){…}` (non-strict); name bound in enclosing statement list; assigned when branch runs | `tests/conformance` fixtures `es/annex-b` |
| E18.11 | done | both | Labelled function declarations (Annex B.3.2): `label: function f() {…}` (non-strict); name hoisted in enclosing statement list; callable | `tests/conformance` fixtures `es/annex-b` |
| E18.10 | done | both | Legacy octal numeric literals (Annex B.1.1): `0[0-7]+` MV octal; NonOctalDecimal `0\d*[89]\d*` MV decimal; no fraction/exp/bigint on pure legacy octal | `tests/conformance` fixtures `es/annex-b` |
| E18.09 | done | both | Legacy octal string escapes (Annex B.1.2): `\0`–`\377` octal sequences; NonOctalDecimal `\8`/`\9` | `tests/conformance` fixtures `es/annex-b` |
| E18.01 | done | both | Global `escape` / `unescape` (Annex B.2.1): `typeof`, `globalThis` identity, basic call behavior | `tests/conformance` fixtures `es/annex-b` |
| E18.02 | done | both | `Object.prototype.__proto__` (Annex B.2.2 / B.3.1): get/set prototype; object literal `__proto__` vs computed `["__proto__"]` | `tests/conformance` fixtures `es/annex-b` |
| E18.03 | done | both | `String.prototype` Annex B (B.2.3): `substr` + HTML wrappers (`anchor`, `big`, `blink`, `bold`, `fixed`, `fontcolor`, `fontsize`, `italics`, `link`, `small`, `strike`, `sub`, `sup`) | `tests/conformance` fixtures `es/annex-b` |
| E18.04 | done | both | `Date.prototype` Annex B (B.2.4–B.2.6): `getYear` / `setYear` / `toGMTString` | `tests/conformance` fixtures `es/annex-b` |
| E18.05 | done | both | `RegExp.prototype.compile` (Annex B.2.6): `typeof`, recompile pattern/flags, read-back `.source`/`.flags`/`.test` | `tests/conformance` fixtures `es/annex-b` |
| E18.06 | done | both | `String.prototype.trimLeft` / `trimRight` (Annex B.2.3): aliases of `trimStart`/`trimEnd`; `typeof`, call behavior, identity with trimStart/trimEnd | `tests/conformance` fixtures `es/annex-b` |
| E18.07 | done | both | `Object.prototype` accessor legacy (B.2.2): `__defineGetter__` / `__defineSetter__` / `__lookupGetter__` / `__lookupSetter__` | `tests/conformance` fixtures `es/annex-b` |
| E18.08 | done | both | HTML-like comments (Annex B.1.3): `<!--` single-line open; line-start `-->` single-line close | `tests/conformance` fixtures `es/annex-b` |

---

## T — Types (Checker; TS-inspired)

| ID | Status | Targets | Item | Tests |
|----|--------|---------|------|-------|
| T01 | done | compiler | Type annotations on bindings and functions | `tests/conformance/types` |
| T02 | done | compiler | Structural object types, type aliases | `tests/conformance/types` |
| T03 | done | compiler | Unions, intersections, narrowing | `tests/conformance/types` |
| T04 | done | compiler | Generics (functions, types) | `tests/conformance/types` |
| T05 | done | compiler | Native types in the type system (`i32`, `i64`, …) | `tests/conformance/types/native` |
| T06 | done | both | Dual-worlds boundary rules (JS value ↔ native) | `tests/conformance/types/dual` |

---

## N — Native types & LLVM

| ID | Status | Targets | Item | Tests |
|----|--------|---------|------|-------|
| N01 | done | native | Integer types `i8`–`i64`, `u8`–`u64` | `tests/conformance/native/ints` |
| N02 | done | native | Floats `f32`/`f64`, native bool | `tests/conformance/native/floats` |
| N03 | done | native | Structs, fixed arrays, pointers/references as designed | `tests/conformance/native/layout` |
| N03.01 | done | native | Native structs: type alias `{ x: i32; … }` of native scalar fields; object literal init; field read `p.x` | `tests/conformance` fixtures `native/layout` |
| N03.02 | done | native | Fixed arrays: tuple type `[T, …]` of native scalars; array literal init; index read `a[i]` (const index) | `tests/conformance` fixtures `native/layout` |
| N03.03 | done | native | Native pointers: type `*T` (T native scalar); address-of `&x`; deref `*p`; store `*p = v` | `tests/conformance` fixtures `native/layout` |
| N04 | done | js | JS lowering/polyfill or hard-error policy per native feature | `tests/conformance/native/js-policy` |
| N05 | todo | native | Link Runtime: GC + minimal std | `crates/draconic-runtime` |
| N06 | todo | native | Async runtime / job queue on native | `tests/conformance/es/async` |
| N07 | todo | native | Embed: compile `eval` strings inside Runtime | `tests/conformance/es/eval` |

---

## Tooling

| ID | Status | Targets | Item | Tests |
|----|--------|---------|------|-------|
| U01 | todo | compiler | `draconic test` runner integration | `crates/draconic-cli` |
| U02 | todo | compiler | Diagnostics: span, message, pretty print | `crates/draconic-diagnostics` |
| U03 | todo | compiler | Source maps for JS emit | `crates/draconic-backend-js` |

---

## How the Loop updates this file

1. Set exactly one item to `in_progress` when claimed.
2. On green tests for that item’s Tests column → `done`.
3. Split a cluster into child rows (e.g. `E01.01`) when the cluster is too large for one Loop — never mark a cluster `done` with failing or missing coverage.
4. Never delete ECMA-262 obligations; move only to finer rows or explicit `blocked` with reason.
