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
| E03 | todo | both | Functions, closures, arguments, arrows (§15) | `tests/conformance/es/functions` |
| E03.01 | done | both | Function declaration + `return` + call (no params) | `tests/conformance` fixtures `es/functions` |
| E04 | todo | both | Objects, prototypes, `this`, property access (§10, §20) | `tests/conformance/es/objects` |
| E05 | todo | both | Classes (§15.7) | `tests/conformance/es/classes` |
| E06 | todo | both | Arrays, iterators, spread/rest | `tests/conformance/es/arrays` |
| E07 | todo | both | Strings, template literals, UTF-16 semantics | `tests/conformance/es/strings` |
| E08 | todo | both | Numbers, BigInt, Math, bitwise | `tests/conformance/es/numbers` |
| E09 | todo | both | Symbols, equality, coercion rules | `tests/conformance/es/values` |
| E10 | todo | both | Exceptions: try/catch/finally/throw | `tests/conformance/es/exceptions` |
| E11 | todo | both | Modules (ESM): import/export, cyclic | `tests/conformance/es/modules` |
| E12 | todo | both | Promises, job queue, async/await | `tests/conformance/es/async` |
| E13 | todo | both | Generators, `yield` | `tests/conformance/es/generators` |
| E14 | todo | both | Proxies, Reflect, exotic objects | `tests/conformance/es/proxies` |
| E15 | todo | both | Realms, globals, built-ins surface | `tests/conformance/es/builtins` |
| E16 | todo | both | `eval`, `new Function`, indirect eval | `tests/conformance/es/eval` |
| E17 | todo | both | `with`, non-strict legacy where required by 262 | `tests/conformance/es/legacy` |
| E18 | todo | both | Remaining Annex B / full 262 gaps (track explicitly, do not drop) | `tests/conformance/es/annex-b` |

---

## T — Types (Checker; TS-inspired)

| ID | Status | Targets | Item | Tests |
|----|--------|---------|------|-------|
| T01 | todo | compiler | Type annotations on bindings and functions | `tests/conformance/types` |
| T02 | todo | compiler | Structural object types, type aliases | `tests/conformance/types` |
| T03 | todo | compiler | Unions, intersections, narrowing | `tests/conformance/types` |
| T04 | todo | compiler | Generics (functions, types) | `tests/conformance/types` |
| T05 | todo | compiler | Native types in the type system (`i32`, `i64`, …) | `tests/conformance/types/native` |
| T06 | todo | both | Dual-worlds boundary rules (JS value ↔ native) | `tests/conformance/types/dual` |

---

## N — Native types & LLVM

| ID | Status | Targets | Item | Tests |
|----|--------|---------|------|-------|
| N01 | todo | native | Integer types `i8`–`i64`, `u8`–`u64` | `tests/conformance/native/ints` |
| N02 | todo | native | Floats `f32`/`f64`, native bool | `tests/conformance/native/floats` |
| N03 | todo | native | Structs, fixed arrays, pointers/references as designed | `tests/conformance/native/layout` |
| N04 | todo | js | JS lowering/polyfill or hard-error policy per native feature | `tests/conformance/native/js-policy` |
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
