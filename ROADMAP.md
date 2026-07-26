# Draconic Roadmap

Source of truth for completeness, together with the test suite.  
**Status**: `todo` | `in_progress` | `done` | `blocked`

A item is `done` only when its tests are green on every applicable target (`js`, `native`, or both).

## Legend

- **Targets**: `js` | `native` | `both` | `compiler` (toolchain-only, no program emit)
- **Tests**: path(s) that must pass
- **Native observations**: `Targets: native`/`both` means fixtures assert **program results** on native (`native.stdout` / equivalent), not the B08 LLVM hello-stub fallback (`hello\n` only)

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
| E01 | done | js | Expressions & operators (ECMA-262 §12–13 core) | `tests/conformance/es/expressions` |
| E01.01 | done | js | Numeric arithmetic: `+` `-` `*` `/` `%`, unary `+`/`-`, grouping/precedence | `tests/conformance` fixtures `es/expressions` |
| E01.02 | done | js | Comparison & equality: `<` `<=` `>` `>=` `==` `!=` `===` `!==` | `tests/conformance` fixtures `es/expressions` |
| E01.03 | done | js | Logical: `&&` `\|\|` `!` | `tests/conformance` fixtures `es/expressions` |
| E01.04 | done | js | Remaining §12–13 (bitwise, assignment, conditional, update, `**`, comma, …) | `tests/conformance` fixtures `es/expressions` |
| E01.04.01 | done | js | Bitwise: `&` `\|` `^` `~` `<<` `>>` `>>>` | `tests/conformance` fixtures `es/expressions` |
| E01.04.02 | done | js | Exponentiation: `**` (right-associative) | `tests/conformance` fixtures `es/expressions` |
| E01.04.03 | done | js | Conditional (ternary): `cond ? a : b` | `tests/conformance` fixtures `es/expressions` |
| E01.04.04 | done | js | Assignment: `=` (simple, right-associative) | `tests/conformance` fixtures `es/expressions` |
| E01.04.05 | done | js | Update: prefix/postfix `++` `--` | `tests/conformance` fixtures `es/expressions` |
| E01.04.06 | done | js | Comma operator: `,` (left-to-right, yields RHS) | `tests/conformance` fixtures `es/expressions` |
| E01.04.07 | done | js | Unary keywords: `typeof` `void` `delete` | `tests/conformance` fixtures `es/expressions` |
| E01.04.08 | done | js | Compound assignment: `+=` `-=` `*=` `/=` `%=` `**=` `<<=` `>>=` `>>>=` `&=` `^=` `\|=` | `tests/conformance` fixtures `es/expressions` |
| E01.04.09 | done | js | Nullish coalescing `??` and logical assignment: `&&=` `\|\|=` `??=` | `tests/conformance` fixtures `es/expressions` |
| E02 | done | js | Statements & control flow (§14) | `tests/conformance/es/statements` |
| E02.01 | done | js | `if` / `else` (incl. block bodies) | `tests/conformance` fixtures `es/statements` |
| E02.02 | done | js | `while` loops (incl. block bodies) | `tests/conformance` fixtures `es/statements` |
| E02.03 | done | js | `do` / `while` loops (incl. block bodies) | `tests/conformance` fixtures `es/statements` |
| E02.04 | done | js | `for` loops: `for (init; test; update)` (incl. `let` init, omitted clauses, block bodies) | `tests/conformance` fixtures `es/statements` |
| E02.05 | done | js | `break` / `continue` (unlabeled, in loops) | `tests/conformance` fixtures `es/statements` |
| E02.06 | done | js | `switch` / `case` / `default` (incl. fall-through, `break`) | `tests/conformance` fixtures `es/statements` |
| E02.07 | done | js | Labeled statements + labeled `break` / `continue` | `tests/conformance` fixtures `es/statements` |
| E02.08 | done | js | `for-in` / `for-of` loops (incl. `let` binding, block bodies; iterate strings) | `tests/conformance` fixtures `es/statements` |
| E02.09 | done | js | `const` declarations (required initializer; no reassignment; `for`/`for-of`/`for-in` binding) | `tests/conformance` fixtures `es/statements` |
| E03 | done | js | Functions, closures, arguments, arrows (§15) | `tests/conformance/es/functions` |
| E03.01 | done | js | Function declaration + `return` + call (no params) | `tests/conformance` fixtures `es/functions` |
| E03.02 | done | js | Function parameters + call with arguments | `tests/conformance` fixtures `es/functions` |
| E03.03 | done | js | Nested function declarations + free-variable capture (outer `let`/params) | `tests/conformance` fixtures `es/functions` |
| E03.04 | done | js | Function expressions: `function (params) { … }` as values (incl. named, IIFE) | `tests/conformance` fixtures `es/functions` |
| E03.05 | done | js | Arrow functions: `(params) => expr` and `(params) => { … }` (simple ident params; no `this`) | `tests/conformance` fixtures `es/functions` |
| E03.06 | done | js | Default parameters: `function f(a = expr)` / `(a = expr) => …` (missing/`undefined` → default) | `tests/conformance` fixtures `es/functions` |
| E03.07 | done | js | Rest parameters: `function f(...args)` / `(...args) => …` (last param; binds array of remaining args) | `tests/conformance` fixtures `es/functions` |
| E04 | done | js | Objects, prototypes, `this`, property access (§10, §20) | `tests/conformance/es/objects` |
| E04.01 | done | js | Object literals `{ k: v }` + property access `obj.k` / `obj["k"]` (read; string keys) | `tests/conformance` fixtures `es/objects` |
| E04.02 | done | js | Property assignment: `obj.k = v` / `obj["k"] = v` (simple `=`; read-back) | `tests/conformance` fixtures `es/objects` |
| E04.03 | done | js | `this` + method call: `obj.m()` / `obj["m"]()` preserves `this`; `function` methods as values | `tests/conformance` fixtures `es/objects` |
| E04.04 | done | js | `new` constructor: `new F(args)` with `function` ctor setting `this` props; returns instance | `tests/conformance` fixtures `es/objects` |
| E04.05 | done | js | Prototypes: `F.prototype.m = function…`; instances inherit methods; `this` in prototype methods | `tests/conformance` fixtures `es/objects` |
| E04.06 | done | js | Object literal sugar: property shorthand `{ a }`, method shorthand `{ m() {…} }`, computed keys `{ [expr]: v }` | `tests/conformance` fixtures `es/objects` |
| E05 | done | js | Classes (§15.7) | `tests/conformance/es/classes` |
| E05.01 | done | js | Class declaration: `class C { constructor(…) {…} m(…) {…} }`; `new C(args)`; instance methods + `this` | `tests/conformance` fixtures `es/classes` |
| E05.02 | done | js | Class heritage: `extends Parent`; `super(…)` in constructor; inherit parent prototype methods | `tests/conformance` fixtures `es/classes` |
| E05.03 | done | js | Static methods: `static m(…) {…}`; call as `C.m(…)` / `new C().constructor.m` not required; no `this` instance binding | `tests/conformance` fixtures `es/classes` |
| E05.04 | done | js | `super` property access: `super.m(…)` / `super.prop` in methods (parent prototype, correct `this`) | `tests/conformance` fixtures `es/classes` |
| E06 | done | js | Arrays, iterators, spread/rest | `tests/conformance/es/arrays` |
| E06.01 | done | js | Array literals `[a, b]` + index access `arr[i]` / `arr.length` (read) | `tests/conformance` fixtures `es/arrays` |
| E06.02 | done | js | Array element assignment: `arr[i] = v` (simple `=`; read-back; length may grow) | `tests/conformance` fixtures `es/arrays` |
| E06.03 | done | js | Spread in array literals: `[...a]` / `[...a, b]` / `[a, ...b, c]` (iterable copy/concat) | `tests/conformance` fixtures `es/arrays` |
| E06.04 | done | js | Spread in call/new args: `f(...a)` / `f(x, ...a, y)` / `new F(...a)` (iterable expand into arguments) | `tests/conformance` fixtures `es/arrays` |
| E06.05 | done | js | `for-of` over arrays: iterate elements; `let`/assign binding; block bodies; nested arrays | `tests/conformance` fixtures `es/arrays` |
| E06.06 | done | js | Array destructuring: `let [a, b] = arr` / `let [a, ...rest] = arr` / assignment `[a, b] = arr` (holes, defaults deferred) | `tests/conformance` fixtures `es/arrays` |
| E07 | done | js | Strings, template literals, UTF-16 semantics | `tests/conformance/es/strings` |
| E07.01 | done | js | String literals (`"`/`'`) + concat `+` + `.length` + index `s[i]` (read; basic escapes `\n` `\r` `\t` `\\` `\'` `\"` `\0`) | `tests/conformance` fixtures `es/strings` |
| E07.02 | done | js | Template literals: `` `…` `` / `` `a${x}b` `` (untagged; cooked escapes; multi-interp) | `tests/conformance` fixtures `es/strings` |
| E07.03 | done | js | Unicode escapes: `\xHH`, `\uXXXX`, `\u{X…}` in string/template literals (cooked; well-formed scalar values) | `tests/conformance` fixtures `es/strings` |
| E07.04 | done | js | Tagged templates: `tag\`…\`` / `tag\`a${x}b\`` (tag call; cooked quasis + interpolations as args) | `tests/conformance` fixtures `es/strings` |
| E07.05 | done | js | UTF-16 code-unit semantics: `.length`/index by code unit; lone surrogates via `\uXXXX`; surrogate-pair escapes equal code-point | `tests/conformance` fixtures `es/strings` |
| E08 | done | js | Numbers, BigInt, Math, bitwise | `tests/conformance/es/numbers` |
| E08.01 | done | js | Number literals: decimal floats, scientific `e`/`E`, hex `0x`, binary `0b`, octal `0o`, numeric separators `_`, leading-dot (`.5`) | `tests/conformance` fixtures `es/numbers` |
| E08.02 | done | js | BigInt integer literals (`1n`, `0xffn`, `0b…n`, `0o…n`, `_` separators) + same-type `+` `-` `*` `/` `%` and unary `-` | `tests/conformance` fixtures `es/numbers` |
| E08.03 | done | js | BigInt comparison & bitwise: `<` `<=` `>` `>=` `==` `!=` `===` `!==` `&` `\|` `^` `~` `<<` `>>` (no `>>>`) | `tests/conformance` fixtures `es/numbers` |
| E08.04 | done | js | BigInt exponentiation: `**` (right-associative) and `**=` (same-type BigInt only) | `tests/conformance` fixtures `es/numbers` |
| E08.05 | done | js | Global `Math`: constants (`E`, `PI`) + methods (`abs`, `floor`, `ceil`, `round`, `min`, `max`, `pow`, `sqrt`, `sign`) via `.` / `[]` and calls | `tests/conformance` fixtures `es/numbers` |
| E08.06 | done | js | Global `NaN` / `Infinity` + `Number`: constants (`NaN`, `POSITIVE_INFINITY`, `NEGATIVE_INFINITY`, `MAX_VALUE`, `MIN_VALUE`, `EPSILON`, `MAX_SAFE_INTEGER`, `MIN_SAFE_INTEGER`) + static methods (`isNaN`, `isFinite`, `isInteger`, `isSafeInteger`) via `.` / `[]` and calls | `tests/conformance` fixtures `es/numbers` |
| E09 | done | js | Symbols, equality, coercion rules | `tests/conformance/es/values` |
| E09.01 | done | js | Symbol constructor basics: `Symbol()` / `Symbol(desc)`, `typeof` `"symbol"`, uniqueness; `Symbol.for` / `Symbol.keyFor` | `tests/conformance` fixtures `es/values` |
| E09.02 | done | js | Symbol property keys: `obj[sym]`, computed keys in literals; own-key read/write (no string collision) | `tests/conformance` fixtures `es/values` |
| E09.03 | done | js | Abstract equality & coercion: `==`/`!=` mixed types; `ToNumber`/`ToString`/`ToBoolean` via `+`/`==`/`if` | `tests/conformance` fixtures `es/values` |
| E09.04 | done | js | `ToPrimitive`: `valueOf` / `toString` hooks in `+` and `==` | `tests/conformance` fixtures `es/values` |
| E10 | done | js | Exceptions: try/catch/finally/throw | `tests/conformance/es/exceptions` |
| E10.01 | done | js | `throw` expression + bare `try`/`catch` (bind catch param; no finally) | `tests/conformance` fixtures `es/exceptions` |
| E10.02 | done | js | `finally`: `try`/`catch`/`finally` and `try`/`finally` (always runs; completion after finally) | `tests/conformance` fixtures `es/exceptions` |
| E10.03 | done | js | Optional catch binding: `catch { … }` (no param; with/without `finally`) | `tests/conformance` fixtures `es/exceptions` |
| E11 | done | js | Modules (ESM): import/export, cyclic | `tests/conformance/es/modules` |
| E11.01 | done | js | Named export + import: `export let`/`const`/`function`, `import { x } from "./mod"` (static relative; no default/star/cycles) | `tests/conformance` fixtures `es/modules` |
| E11.02 | done | js | Default export + import: `export default …` / `export default function…`, `import d from "./mod"` / `import d, { x } from "./mod"` (static relative; no star/cycles) | `tests/conformance` fixtures `es/modules` |
| E11.03 | done | js | Namespace import: `import * as ns from "./mod"` / `import d, * as ns from "./mod"` (static relative; ns props are named exports + `default` when present; no cycles) | `tests/conformance` fixtures `es/modules` |
| E11.04 | done | js | Cyclic modules: mutual static relative imports; named exports as functions + live `let` bindings (no namespace/`export *` in cycle fixtures) | `tests/conformance` fixtures `es/modules` |
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
| E13 | done | js | Generators, `yield` | `tests/conformance/es/generators` |
| E13.01 | done | js | Generator function declaration: `function* g() { yield expr; return expr; }`; call returns iterator; `.next()` → `{value, done}` | `tests/conformance` fixtures `es/generators` |
| E13.02 | done | js | Yield expression RHS: AssignmentExpression-level (not unary); bare `yield` → undefined; multiple sequential yields; generator params | `tests/conformance` fixtures `es/generators` |
| E13.03 | done | js | Yield resume: `.next(arg)` becomes the value of the paused `yield` expr; first `.next()` arg ignored; bare/`yield expr` both resume | `tests/conformance` fixtures `es/generators` |
| E13.04 | done | js | `yield*`: delegate to iterable/iterator (`yield* expr`); flatten nested generators; completion value of inner | `tests/conformance` fixtures `es/generators` |
| E13.05 | done | js | Generator function expressions: `function* (params) { … }` as values (incl. named, IIFE); call returns iterator | `tests/conformance` fixtures `es/generators` |
| E13.06 | done | js | Generator methods: object `{ *m() {…} }` / class `*m()` / `static *m()`; call returns iterator; `yield` in method body | `tests/conformance` fixtures `es/generators` |
| E13.07 | done | js | `for-of` over generators: iterate yielded values; `let`/assign binding; block bodies; early `break` | `tests/conformance` fixtures `es/generators` |
| E13.08 | done | js | Generator `.return(value)` / `.throw(exception)`: close with value; inject exception at paused `yield`; try/catch in body | `tests/conformance` fixtures `es/generators` |
| E14 | done | js | Proxies, Reflect, exotic objects | `tests/conformance/es/proxies` |
| E14.01 | done | js | Proxy constructor basics: `new Proxy(target, handler)`; empty-handler pass-through get; `get` trap; `typeof Proxy` | `tests/conformance` fixtures `es/proxies` |
| E14.02 | done | js | Proxy `set` trap: empty-handler pass-through set; `set` trap intercept write; read-back | `tests/conformance` fixtures `es/proxies` |
| E14.03 | done | js | Proxy `has` trap + `in`: empty-handler pass-through `"k" in obj`; `has` trap intercept; plain object `in` | `tests/conformance` fixtures `es/proxies` |
| E14.04 | done | js | Proxy `deleteProperty` trap + `delete`: empty-handler pass-through `delete obj.k`; trap intercept; plain object `delete` | `tests/conformance` fixtures `es/proxies` |
| E14.05 | done | js | Proxy `apply` trap: empty-handler pass-through call; `apply` trap intercept call; callable target | `tests/conformance` fixtures `es/proxies` |
| E14.06 | done | js | Proxy `construct` trap: empty-handler pass-through `new`; `construct` trap intercept `new`; constructable target | `tests/conformance` fixtures `es/proxies` |
| E14.07 | done | js | Reflect basics: `typeof Reflect`; `Reflect.get`/`set`/`has`/`deleteProperty`/`apply`/`construct` on plain objects + Proxy targets | `tests/conformance` fixtures `es/proxies` |
| E14.08 | done | js | Proxy `ownKeys` trap + `Reflect.ownKeys`: empty-handler pass-through; trap intercept; plain object keys | `tests/conformance` fixtures `es/proxies` |
| E14.09 | done | js | Proxy `getPrototypeOf`/`setPrototypeOf` traps + `Reflect.getPrototypeOf`/`setPrototypeOf`: empty-handler pass-through; trap intercept; plain object | `tests/conformance` fixtures `es/proxies` |
| E14.10 | done | js | Proxy `defineProperty`/`getOwnPropertyDescriptor` traps + `Reflect.defineProperty`/`getOwnPropertyDescriptor`: empty-handler pass-through; trap intercept; plain object data descriptors | `tests/conformance` fixtures `es/proxies` |
| E14.11 | done | js | Proxy `isExtensible`/`preventExtensions` traps + `Reflect.isExtensible`/`preventExtensions`: empty-handler pass-through; trap intercept; plain object | `tests/conformance` fixtures `es/proxies` |
| E15 | done | js | Realms, globals, built-ins surface | `tests/conformance/es/builtins` |
| E15.01 | done | js | Global object basics: `undefined`, `globalThis`, fundamental constructors `Object`/`Function`/`Array`/`String`/`Boolean` (`typeof`) | `tests/conformance` fixtures `es/builtins` |
| E15.02 | done | js | Error constructors: `Error` / `TypeError` / `RangeError` / `ReferenceError` / `SyntaxError` / `URIError` / `EvalError` / `AggregateError` (`typeof`, `globalThis` identity, `new …(msg)`, `.name`/`.message`) | `tests/conformance` fixtures `es/builtins` |
| E15.03 | done | js | Global functions: `parseInt` / `parseFloat` / `isNaN` / `isFinite` (`typeof`, `globalThis` identity, basic call behavior) | `tests/conformance` fixtures `es/builtins` |
| E15.04 | done | js | URI encode/decode: `encodeURI` / `decodeURI` / `encodeURIComponent` / `decodeURIComponent` (`typeof`, `globalThis` identity, basic call behavior) | `tests/conformance` fixtures `es/builtins` |
| E15.05 | done | js | Global `JSON`: `typeof JSON`, `globalThis` identity, `JSON.parse` / `JSON.stringify` basics | `tests/conformance` fixtures `es/builtins` |
| E15.06 | done | js | Global `Date`: `typeof Date`, `globalThis` identity, `Date.now`, `new Date(ms)` / `.getTime()` / `.valueOf()` basics | `tests/conformance` fixtures `es/builtins` |
| E15.07 | done | js | Global `RegExp`: `typeof RegExp`, `globalThis` identity, `new RegExp(pattern)` / `new RegExp(pattern, flags)`, `.test` / `.exec` / `.source` / `.flags` basics | `tests/conformance` fixtures `es/builtins` |
| E15.08 | done | js | Global `Map` / `Set`: `typeof`, `globalThis` identity, `new Map` / `new Set`, `.set`/`.get`/`.has`/`.size` and `.add`/`.has`/`.size` basics | `tests/conformance` fixtures `es/builtins` |
| E15.09 | done | js | Global `WeakMap` / `WeakSet`: `typeof`, `globalThis` identity, `new WeakMap` / `new WeakSet`, `.set`/`.get`/`.has`/`.delete` and `.add`/`.has`/`.delete` (object keys only) | `tests/conformance` fixtures `es/builtins` |
| E15.10 | done | js | Global `ArrayBuffer` / `DataView` / TypedArrays: `typeof`, `globalThis` identity, `new ArrayBuffer(n)` / `.byteLength`, `new Uint8Array`/`Int32Array`/`Float64Array`, `.length`/index read-write, `new DataView(buf)` / `.getUint8`/`.setUint8` | `tests/conformance` fixtures `es/builtins` |
| E16 | done | both | `eval`, `new Function`, indirect eval | `tests/conformance/es/eval` |
| E16.01 | done | both | Direct `eval`: `typeof eval`, `globalThis` identity, `eval(string)` expression/statement basics | `tests/conformance` fixtures `es/eval` |
| E16.02 | done | both | `new Function` / `Function(...)`: construct from strings; call returns function; simple body/params | `tests/conformance` fixtures `es/eval` |
| E16.03 | done | both | Indirect eval: `(0, eval)(s)` / `globalThis.eval(s)` (global scope; not caller lexical) | `tests/conformance` fixtures `es/eval` |
| E17 | done | js | `with` (E17.01); other non-strict legacy → E17.02 | `tests/conformance/es/legacy` |
| E17.01 | done | js | `with` statement: object binding in scope; property read/write; nested `with` | `tests/conformance` fixtures `es/legacy` |
| E17.02 | todo | js | Other non-strict legacy required by ECMA-262 beyond `with` (file finer rows as discovered) | `tests/conformance/es/legacy` |
| E18 | done | js | Annex B / late ES features tracked as children below (remainder → E18.44) | `tests/conformance/es/annex-b` |
| E18.44 | todo | js | Untracked ECMA-262 remainder beyond E01–E18 children (file finer rows as discovered; do not drop) | `tests/conformance` (new fixtures as filed) |
| E18.43 | done | js | Async generators: `async function* g(){ yield expr; }` / `async function* (params){…}` / `{ async *m(){…} }` / class `async *m()`; call returns async iterator; `.next()` → Promise of `{value,done}`; `for await` over async gen | `tests/conformance` fixtures `es/annex-b` |
| E18.42 | done | js | `for await…of`: `for await (let x of asyncIterable)` / assign binding; async iter protocol; only in async functions | `tests/conformance` fixtures `es/annex-b` |
| E18.41 | done | js | Class static initialization blocks: `static { … }` (runs at class eval; access private statics; multiple blocks in order) | `tests/conformance` fixtures `es/annex-b` |
| E18.40 | done | js | Private brand check: `#x in obj` (true iff obj has the private field/method/accessor; false for null/undefined/unrelated) | `tests/conformance` fixtures `es/annex-b` |
| E18.39 | done | js | Private accessors: `get #x()` / `set #x(v)` / `static get`/`set #x`; read/write via `this.#x` / `C.#x` (not public; not on prototype) | `tests/conformance` fixtures `es/annex-b` |
| E18.38 | done | js | Static private methods: `class C { static #m(…){…} }` / call `C.#m(…)` / `this.#m(…)` in static (not on instances; not public) | `tests/conformance` fixtures `es/annex-b` |
| E18.37 | done | js | Private instance methods: `class C { #m(…){…} }` / call `this.#m(…)` (not on prototype; not public) | `tests/conformance` fixtures `es/annex-b` |
| E18.35 | done | js | Private instance fields: `class C { #x = expr; #y; m(){ this.#x } }` / assign `this.#x = v` (simple `=`; not public props) | `tests/conformance` fixtures `es/annex-b` |
| E18.36 | done | js | Static private fields: `class C { static #x = expr; static #y; m(){ C.#x } }` / assign `C.#x = v` (class-side; not on instances) | `tests/conformance` fixtures `es/annex-b` |
| E18.34 | done | js | Async methods: object `{ async m(){…} }` / class `async m()` / `static async m()`; `await` in body; returns Promise | `tests/conformance` fixtures `es/annex-b` |
| E18.33 | done | js | Class expressions: `let C = class {…}` / `let C = class Name {…}` (methods, `extends`, fields; `new C`) | `tests/conformance` fixtures `es/annex-b` |
| E18.32 | done | js | `export class C` / `export default class C`: named class export and default; consumer `import { C }` / `import C from` | `tests/conformance` fixtures `es/annex-b` |
| E18.31 | done | js | `export * as ns from`: namespace re-export from static relative module; consumer `import { ns }` / `import * as m` sees `ns` as module namespace object | `tests/conformance` fixtures `es/annex-b` |
| E18.30 | done | js | `export { x } from` / `export { x as y } from` / `export { default as d } from`: named re-export from static relative module | `tests/conformance` fixtures `es/annex-b` |
| E18.29 | done | js | `export * from`: re-export all named exports from a static relative module; `import { x } from` consumer sees re-exported names | `tests/conformance` fixtures `es/annex-b` |
| E18.28 | done | js | Object spread in literals: `{...a}` / `{...a, b}` / `{a, ...b, c}` (copy enumerable own string keys; later props overwrite) | `tests/conformance` fixtures `es/annex-b` |
| E18.27 | done | js | `new.target`: meta-property in `function`/`constructor`; `undefined` in non-`new` call; subclass points at active construct | `tests/conformance` fixtures `es/annex-b` |
| E18.26 | done | js | Class public fields: `class C { x = expr; static y = expr; }` (simple ident; instance after `super`; optional init → undefined) | `tests/conformance` fixtures `es/annex-b` |
| E18.25 | done | js | Parameter destructuring: `function f({a})` / `function f([a])` / arrows; rename, nested, rest, defaults | `tests/conformance` fixtures `es/annex-b` |
| E18.24 | done | js | `arguments` object: `arguments[i]` / `arguments.length` in non-arrow `function` (call arity; not arrow) | `tests/conformance` fixtures `es/annex-b` |
| E18.23 | done | js | Optional chaining: `a?.b` / `a?.[expr]` / `a?.(args)` (short-circuit on `null`/`undefined`; chainable) | `tests/conformance` fixtures `es/annex-b` |
| E18.22 | done | js | Accessor properties: object/class `get name(){}` / `set name(v){}` (incl. `static get`/`set`; read/write via property access) | `tests/conformance` fixtures `es/annex-b` |
| E18.21 | done | js | `instanceof`: `obj instanceof Ctor` (function/class constructors; prototype chain) | `tests/conformance` fixtures `es/annex-b` |
| E18.20 | done | js | Destructuring defaults: `let [a = expr] = arr` / `let {a = expr} = obj` / `{a: b = expr}` / assignment patterns with defaults | `tests/conformance` fixtures `es/annex-b` |
| E18.19 | done | js | Object destructuring: `let {a, b} = obj` / `let {a, ...rest} = obj` / assignment `{a, b} = obj` (rename, nested; defaults deferred) | `tests/conformance` fixtures `es/annex-b` |
| E18.18 | done | js | Regular expression literals: `/pattern/` / `/pattern/flags`; `typeof` `"object"`; `.source`/`.flags`/`.test`/`.exec` parity with `new RegExp` | `tests/conformance` fixtures `es/annex-b` |
| E18.17 | done | js | VariableStatements in Catch (Annex B.3.4): `catch (e) { var e … }` allowed; var hoists to VariableEnvironment; initializer assigns catch binding | `tests/conformance` fixtures `es/annex-b` |
| E18.16 | done | js | RegExp constructor Annex B statics (B.2.5): `$1`–`$9`, `input`/`$_`, `lastMatch`/`$&`, `lastParen`/`$+`, `leftContext`/`$\``, `rightContext`/`$'` after match/exec | `tests/conformance` fixtures `es/annex-b` |
| E18.15 | done | js | `var` in `for` heads: `for (var i=…;…;…)`, `for (var k in/of …)`, Annex B.3.5 `for (var k = init in obj)` | `tests/conformance` fixtures `es/annex-b` |
| E18.14 | done | js | `var` declarations: `var x` / `var x = expr` (function-scoped hoist; redeclaration; no TDZ; simple ident) | `tests/conformance` fixtures `es/annex-b` |
| E18.13 | done | js | Block-level function declarations (Annex B.3.2): `{ function f(){…} }` (non-strict); name block-local + enclosing var-like binding; assigned when block runs | `tests/conformance` fixtures `es/annex-b` |
| E18.12 | done | js | FunctionDeclarations in `if` (Annex B.3.4): `if (c) function f(){…}` / `else function f(){…}` (non-strict); name bound in enclosing statement list; assigned when branch runs | `tests/conformance` fixtures `es/annex-b` |
| E18.11 | done | js | Labelled function declarations (Annex B.3.2): `label: function f() {…}` (non-strict); name hoisted in enclosing statement list; callable | `tests/conformance` fixtures `es/annex-b` |
| E18.10 | done | js | Legacy octal numeric literals (Annex B.1.1): `0[0-7]+` MV octal; NonOctalDecimal `0\d*[89]\d*` MV decimal; no fraction/exp/bigint on pure legacy octal | `tests/conformance` fixtures `es/annex-b` |
| E18.09 | done | js | Legacy octal string escapes (Annex B.1.2): `\0`–`\377` octal sequences; NonOctalDecimal `\8`/`\9` | `tests/conformance` fixtures `es/annex-b` |
| E18.01 | done | js | Global `escape` / `unescape` (Annex B.2.1): `typeof`, `globalThis` identity, basic call behavior | `tests/conformance` fixtures `es/annex-b` |
| E18.02 | done | js | `Object.prototype.__proto__` (Annex B.2.2 / B.3.1): get/set prototype; object literal `__proto__` vs computed `["__proto__"]` | `tests/conformance` fixtures `es/annex-b` |
| E18.03 | done | js | `String.prototype` Annex B (B.2.3): `substr` + HTML wrappers (`anchor`, `big`, `blink`, `bold`, `fixed`, `fontcolor`, `fontsize`, `italics`, `link`, `small`, `strike`, `sub`, `sup`) | `tests/conformance` fixtures `es/annex-b` |
| E18.04 | done | js | `Date.prototype` Annex B (B.2.4–B.2.6): `getYear` / `setYear` / `toGMTString` | `tests/conformance` fixtures `es/annex-b` |
| E18.05 | done | js | `RegExp.prototype.compile` (Annex B.2.6): `typeof`, recompile pattern/flags, read-back `.source`/`.flags`/`.test` | `tests/conformance` fixtures `es/annex-b` |
| E18.06 | done | js | `String.prototype.trimLeft` / `trimRight` (Annex B.2.3): aliases of `trimStart`/`trimEnd`; `typeof`, call behavior, identity with trimStart/trimEnd | `tests/conformance` fixtures `es/annex-b` |
| E18.07 | done | js | `Object.prototype` accessor legacy (B.2.2): `__defineGetter__` / `__defineSetter__` / `__lookupGetter__` / `__lookupSetter__` | `tests/conformance` fixtures `es/annex-b` |
| E18.08 | done | js | HTML-like comments (Annex B.1.3): `<!--` single-line open; line-start `-->` single-line close | `tests/conformance` fixtures `es/annex-b` |

---

## T — Types (Checker; TS-inspired)

| ID | Status | Targets | Item | Tests |
|----|--------|---------|------|-------|
| T01 | done | compiler | Type annotations on bindings and functions | `tests/conformance/types` |
| T02 | done | compiler | Structural object types, type aliases | `tests/conformance/types` |
| T03 | done | compiler | Unions, intersections, narrowing | `tests/conformance/types` |
| T04 | done | compiler | Generics (functions, types) | `tests/conformance/types` |
| T05 | done | compiler | Native types in the type system (`i32`, `i64`, …) | `tests/conformance/types/native` |
| T06 | done | js | Dual-worlds boundary rules (JS value ↔ native) | `tests/conformance/types/dual` |
| T07 | todo | compiler | Negative typechecking: reject ill-typed programs with diagnostics (not erase-only happy paths) | `tests/conformance/types` (reject fixtures) |

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
| N05 | done | native | Link Runtime: GC + minimal std | `crates/draconic-runtime` |
| N06 | done | native | Async runtime / job queue on native | `tests/conformance/es/async` |
| N06.01 | done | native | Job queue Runtime ABI: enqueue host jobs + drain FIFO (nested enqueue during drain runs after current job) | `crates/draconic-runtime` |
| N06.02 | done | native | Runtime Promise ABI: construct, sync resolve/reject, one-hop `then` reactions via job queue (FIFO drain) | `crates/draconic-runtime` |
| N06.03 | done | native | LLVM Promise basics via Runtime ABI: `new Promise(executor)`, one-hop `.then`, end-of-main `job_drain`; real native observations for `es/async/promise_basics` | `tests/conformance` fixtures `es/async/promise_basics`, `crates/draconic-backend-llvm`, `crates/draconic-runtime` |
| N06.04 | done | native | LLVM Promise statics + catch via Runtime ABI: `Promise.resolve` / `Promise.reject`, `.catch`; real native observations for `es/async/promise_resolve_reject` | `tests/conformance` fixtures `es/async/promise_resolve_reject`, `crates/draconic-backend-llvm`, `crates/draconic-runtime` |
| N06.05 | done | native | LLVM `Promise.prototype.finally` via Runtime ABI: fulfill + reject paths; value/reason pass-through; real native observations for `es/async/promise_finally` | `tests/conformance` fixtures `es/async/promise_finally`, `crates/draconic-backend-llvm`, `crates/draconic-runtime` |
| N06.06 | done | native | LLVM `Promise.all` via Runtime ABI: iterable of promises/values; fulfill with array; reject on first rejection; empty → `[]`; real native observations for `es/async/promise_all` | `tests/conformance` fixtures `es/async/promise_all`, `crates/draconic-backend-llvm`, `crates/draconic-runtime` |
| N06.07 | done | native | LLVM `Promise.race` via Runtime ABI: iterable of promises/values; settle with first fulfillment or rejection; real native observations for `es/async/promise_race` | `tests/conformance` fixtures `es/async/promise_race`, `crates/draconic-backend-llvm`, `crates/draconic-runtime` |
| N06.08 | done | native | LLVM `Promise.allSettled` via Runtime ABI: iterable of promises/values; fulfill with `{status,value\|reason}[]`; empty → `[]`; real native observations for `es/async/promise_allSettled` | `tests/conformance` fixtures `es/async/promise_allSettled`, `crates/draconic-backend-llvm`, `crates/draconic-runtime` |
| N06.09 | done | native | LLVM `Promise.any` via Runtime ABI: iterable of promises/values; fulfill with first fulfillment; reject `AggregateError` if all reject; empty → reject; real native observations for `es/async/promise_any` | `tests/conformance` fixtures `es/async/promise_any`, `crates/draconic-backend-llvm`, `crates/draconic-runtime` |
| N06.10 | done | native | LLVM `async function` + `await` via Runtime ABI: declaration/expression, `await` expr; returns Promise; sync throw → reject; real native observations for `es/async/async_await` | `tests/conformance` fixtures `es/async/async_await`, `crates/draconic-backend-llvm`, `crates/draconic-runtime` |
| N06.11 | done | native | LLVM async arrows via Runtime ABI: `async (params) => expr` / `async (params) => { … }` (simple ident params; `await` in body; returns Promise); real native observations for `es/async/async_arrow` | `tests/conformance` fixtures `es/async/async_arrow`, `crates/draconic-backend-llvm`, `crates/draconic-runtime` |
| N07 | done | native | Embed: compile `eval` strings inside Runtime | `tests/conformance/es/eval` |
| N07.01 | done | native | Embed: compile + eval simple expression strings (number/string literals, arithmetic `+` `-` `*` `/` `%`, unary `+/-`, grouping, `typeof` on primitives/`undefined`) via Frontend→IR interpreter | `crates/draconic-embed` |
| N07.02 | done | native | LLVM direct `eval` via Embed: constant-string `eval(...)` folded through Embed at emit; `typeof eval`; `globalThis.eval === eval`; real native observations for `es/eval/direct_eval` | `tests/conformance` fixtures `es/eval/direct_eval`, `crates/draconic-backend-llvm`, `crates/draconic-embed` |
| N07.03 | done | native | LLVM `new Function` / `Function(...)` via Embed: constant-string params/body folded at emit into callables; `typeof Function`; `globalThis.Function === Function`; call with args; real native observations for `es/eval/new_function` | `tests/conformance` fixtures `es/eval/new_function`, `crates/draconic-backend-llvm`, `crates/draconic-embed` |
| N07.04 | done | native | LLVM indirect eval via Embed: `(0, eval)(s)` / `globalThis.eval(s)` global scope (not caller lexical); direct `eval` still lexical; real native observations for `es/eval/indirect_eval` | `tests/conformance` fixtures `es/eval/indirect_eval`, `crates/draconic-backend-llvm`, `crates/draconic-embed` |
| N08 | todo | native | Real native observations for ES clusters still on B08 hello stub (not stub-green) | `tests/conformance` (update `native.stdout` off hello) |
| N08.01 | todo | native | Real native observations: expressions (E01) — LLVM path asserts program results, not B08 hello | `tests/conformance` fixtures `es/expressions` |
| N08.02 | todo | native | Real native observations: statements (E02) | `tests/conformance` fixtures `es/statements` |
| N08.03 | todo | native | Real native observations: functions (E03) | `tests/conformance` fixtures `es/functions` |
| N08.04 | todo | native | Real native observations: objects (E04) | `tests/conformance` fixtures `es/objects` |
| N08.05 | todo | native | Real native observations: classes (E05) | `tests/conformance` fixtures `es/classes` |
| N08.06 | todo | native | Real native observations: arrays (E06) | `tests/conformance` fixtures `es/arrays` |
| N08.07 | todo | native | Real native observations: strings (E07) | `tests/conformance` fixtures `es/strings` |
| N08.08 | todo | native | Real native observations: numbers/BigInt/Math (E08) | `tests/conformance` fixtures `es/numbers` |
| N08.09 | todo | native | Real native observations: symbols/equality/coercion (E09) | `tests/conformance` fixtures `es/values` |
| N08.10 | todo | native | Real native observations: exceptions (E10) | `tests/conformance` fixtures `es/exceptions` |
| N08.11 | todo | native | Real native observations: modules (E11) | `tests/conformance` fixtures `es/modules` |
| N08.12 | todo | native | Real native observations: generators (E13) | `tests/conformance` fixtures `es/generators` |
| N08.13 | todo | native | Real native observations: proxies/Reflect (E14) | `tests/conformance` fixtures `es/proxies` |
| N08.14 | todo | native | Real native observations: builtins surface (E15) | `tests/conformance` fixtures `es/builtins` |
| N08.15 | todo | native | Real native observations: legacy `with` (E17) | `tests/conformance` fixtures `es/legacy` |
| N08.16 | todo | native | Real native observations: annex-b / late ES (E18 children) | `tests/conformance` fixtures `es/annex-b` |
| N08.17 | todo | native | Real native observations: dual-worlds boundary (T06) | `tests/conformance` fixtures `types/dual` |

---

## Tooling

| ID | Status | Targets | Item | Tests |
|----|--------|---------|------|-------|
| U01 | done | compiler | `draconic test` runner integration | `crates/draconic-cli` |
| U02 | done | compiler | Diagnostics: span, message, pretty print | `crates/draconic-diagnostics` |
| U03 | done | compiler | Source maps for JS emit | `crates/draconic-backend-js` |

---

## How the Loop updates this file

1. Set exactly one item to `in_progress` when claimed.
2. On green tests for that item’s Tests column → `done`.
3. Split a cluster into child rows (e.g. `E01.01`) when the cluster is too large for one Loop — never mark a cluster `done` with failing or missing coverage.
4. Never delete ECMA-262 obligations; move only to finer rows or explicit `blocked` with reason.
