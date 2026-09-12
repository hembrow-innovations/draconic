---
id: "guides-typescript-gaps"
title: "TypeScript gaps in Draconic"
kind: guide
description: "What TypeScript has that Draconic does not: Checker surface, compiler product, and editor ecosystem, versus locked non-goals."
domain: draconic
area: types
tags: [guide, types, typescript, checker]
created_at: "2026-09-12"
updated_at: "2026-09-12"
---

# TypeScript gaps in Draconic

## Overview

This guide is a reading list of TypeScript features and product surfaces that Draconic does not have. It is not a backlog. The Checker is TypeScript-inspired and not tsc-compatible ([[0005-ts-inspired-not-tsc]], [[CONTEXT]] Checker). Roadmap **T01–T07.05** are `done`. Do not treat anything here as a Loop item unless [[ROADMAP]] later marks it `todo`.

Draconic source looks like a typed ECMAScript superset. The JS backend emits JavaScript, not TypeScript. Drop-in migration of a TypeScript repo is out of scope so Native types and Dual worlds are not constrained by tsc’s type-erasure model.

## Prerequisites

- **Stance**: [[0005-ts-inspired-not-tsc]] and [[specs/draconic/language/types/contract]]. Untyped JavaScript staying permissive is not a gap.
- **What is locked**: [[architecture-check]] and Roadmap track **T**.
- **Terms**: [[CONTEXT]] — Checker, Dual worlds, Native type, JS value, Program.
- **Not this note**: ECMA-262 remainder lives on **E** in [[ROADMAP]]. Host I/O, packages, and LLVM are not TypeScript gaps.

## Steps

1. **Read the stance.** Draconic will not become tsc. Missing `tsconfig` flags, `.d.ts` emit, and DefinitelyTyped are product nos, not unfinished clones.
2. **Read the locked Checker.** Annotations, structural shapes, aliases, unions, intersections, `typeof` narrowing, simple generics, and annotated reject diagnostics are the done bar. Everything else in TypeScript’s handbook is extra relative to that bar.
3. **Walk the type-system catalog** below (syntax, lattice, type-level programming, classes).
4. **Walk the product catalog** (modules and declarations, compiler, editors).
5. **Stop at the explicit nos.** If a feature would only exist to compile existing TypeScript, it does not belong on [[ROADMAP]] without a human decision.

## What Draconic already locks

These are present and tested. They are the baseline for “inspired by TypeScript,” not a claim of tsc.

- **Annotations**: Bindings, parameters, and function or arrow returns may carry a type. Mismatched init, assign, default, or return is a diagnostic ([[specs/draconic/language/types/contract]] `language.types:annotations` and `mismatch-diagnostics`).
- **Structural object types and aliases**: `{ a: T; b: U }` and `type Name = …` / `type Name<T> = …`. Matching literals assign; missing or wrong properties do not.
- **Unions, intersections, narrowing**: `A | B`, `A & B`. Unions narrow on `typeof` checks. That is the promised narrowing, not TypeScript’s full control-flow graph.
- **Generics**: Generic aliases and generic functions. Call sites infer type arguments. Arity and argument mismatches reject.
- **Call-site checking (annotated only)**: Wrong arity and non-assignable arguments reject. Unannotated parameters stay permissive. Rest parameters allow extra arguments.
- **Annotated honesty**: Missing return in a non-void annotated function; unknown property on an annotated shape; excess property on a fresh literal vs an annotated shape; call or `new` of an annotated non-callable.
- **Named builtins the Checker knows**: `number`, `string`, `boolean`, `bigint`, `any`, `null`, `object`, `function`, plus native names `i8`–`i64`, `u8`–`u64`, `f32`, `f64`, `bool`.
- **Annotation AST**: Named, generic application `Foo<T>`, object, tuple `[T, U]`, pointer `*T`, union, intersection. That is the whole `TypeAnn` enum.
- **Checker lattice**: `Number`, `BigInt`, `String`, `Boolean`, `Null`, `Function`, `Object`, `Shape`, `Union`, `Intersection`, `TypeParam`, `GenericFn`, `Native`, `Ptr`, `Any`. There is no `never`, `unknown`, `void`, `undefined`, `symbol`, or literal type in that enum.
- **Dual-world `as`**: JS `number` ↔ unboxed native numeric (not native `bool`). Other non-assignable `as` pairs error. This is not tsc type assertion.
- **Parse extras that are not a full type system**: `import type` / type-only import forms exist on the AST. `type` aliases erase at emit. That is not declaration emit or `.d.ts`.

Untyped programs typecheck like dynamic JavaScript. Annotations opt into the T07 rules.

## Type syntax TypeScript has

Surface forms people write in `.ts` that Draconic does not treat as Checker features.

- **`interface`**: TypeScript’s primary named object type, with declaration merging, `extends`, and `implements`. In Draconic, `interface` is only a strict-mode reserved word (ECMA FutureReservedWord). There is no interface declaration, no merging, no interface `extends`.
- **`enum` / `const enum`**: Numeric and string enums, reverse mapping, const-enum inlining. `enum` is an ECMA ReservedWord here, not a TypeScript enum feature.
- **`namespace` / `module` (ambient)**: TypeScript namespaces, nested exports, and `export as namespace`. Draconic modules are ESM. There is no TS namespace type.
- **Optional properties**: `{ x?: T }`. Draconic `TypeProp` is `name: Type` only. No `?` on type properties.
- **Readonly properties**: `readonly x: T` in types and `Readonly<T>`. Not in `TypeAnn` or shapes.
- **Index signatures**: `{ [key: string]: T }`, number indexes, template-index signatures. Shapes are a closed list of named props.
- **Function type syntax**: `(a: T) => U`, constructor types `new (a: T) => U`, `this` parameters. Callables collapse to `Type::Function` (or `GenericFn` at generic declarations). Parameter and return types live on annotated bindings as `FnSig` for call checking; they are not first-class function types you can alias, union, or pass around as `(x: number) => string`.
- **Method and call signatures in object types**: `{ m(x: T): U }`, `{ (x: T): U }`. Object types are data properties only.
- **Tuple labels and optionality**: `[x: T, y?: U]`, rest tuples `[...T[]]`, variadic tuples. Draconic tuples intern as non-strict shapes with `"0"`, `"1"`, … keys (native fixed arrays on the native path). They are not TypeScript tuple types.
- **Parenthesized and operator types**: `typeof`, `keyof`, `infer`, `readonly` type operators, `unique symbol`, import types `import("mod").Foo`. None of these are `TypeAnn` variants.
- **`asserts` / predicate signatures**: `x is T`, `asserts x is T`. No type predicates.
- **`satisfies`**: TypeScript 4.9 operator. Not parsed as a Checker feature.
- **`as const`**: Literal widening lock. Dual-world `as` is a conversion boundary, not const assertion.
- **`as` type assertion (tsc)**: TypeScript `as T` / angle-bracket assertion is a compile-time lie that erases. Draconic `as` is Dual worlds. Do not read tsc assertion docs as product law ([[architecture-check]]).
- **Angle-bracket assertions and JSX overlap**: `<T>expr`. Not a Draconic type form.
- **Decorators (stage / experimental)**: Parameter, method, class decorators and metadata. Not a Checker product.
- **Parameter properties**: `constructor(public x: T)`. Not present.
- **Ambient `declare`**: `declare const`, `declare function`, `declare class`, `declare module`, `declare global`, `declare namespace`. No ambient declaration space.
- **Triple-slash directives**: `/// <reference path="…" />`, `types`, `lib`, `amd-module`. No `tsconfig` and no reference directives.

## Type lattice TypeScript has

Types that exist in tsc’s universe and not in `draconic-check`’s `Type` enum.

- **`undefined`**: Distinct from `null` in TypeScript (especially under `strictNullChecks`). Draconic has `Null` and `Any`. There is no `Undefined` variant.
- **`void`**: “Callable, ignore the return.” Annotated functions can be non-void vs missing-return, but `void` is not a lattice type you write and compose.
- **`never`**: Bottom type, exhaustive checks, unreachable. Empty unions currently collapse toward `Any`, not `never`.
- **`unknown`**: Top type that must be narrowed before use. Draconic’s permissive top is `Any` (unannotated JS).
- **`symbol` / `unique symbol`**: Runtime `Symbol` exists on the ECMA path. The Checker does not model `symbol` as a type.
- **Literal types**: `"foo"`, `42`, `true`, numeric/string/bigint literals as types. No literal lattice, so no discriminated unions in the TypeScript sense (tag literals).
- **Template literal types**: `` `id-${string}` ``, inference into template positions.
- **Indexed access**: `T["k"]`, `T[K]`.
- **Conditional types**: `T extends U ? X : Y`, including distributive conditionals.
- **`infer`**: Pulling types out of conditionals (`T extends (...args: infer P) => any ? P : never`).
- **Mapped types**: `{ [K in keyof T]: U }`, `readonly` / optional modifiers in mapped types, `-readonly`, `-?`.
- **`keyof`**: Key union of a shape. Shapes have props internally; there is no `keyof` operator.
- **`typeof` as a type query**: `typeof value` in type position. Runtime `typeof` narrowing in `if` is implemented; type-position `typeof` is not.
- **Recursive type aliases (tsc-depth)**: TypeScript allows `type Json = string | number | { [k: string]: Json }` with a large solver. Draconic aliases resolve bodies; there is no claimed recursive-conditional solver.
- **Branded / nominal helpers**: TypeScript encodes brands with intersections of unique literals. No literal types means that encoding is not available. Native types are actually nominal in another sense (unboxed, not JS values) — that is Dual worlds, not TS branding.
- **Utility types (lib.es)**: `Partial`, `Required`, `Readonly`, `Pick`, `Omit`, `Record`, `Exclude`, `Extract`, `NonNullable`, `Parameters`, `ReturnType`, `InstanceType`, `ThisParameterType`, `Awaited`, `Uppercase` / `Lowercase` / `Capitalize`, and the rest. None ship as Checker builtins.
- **`Promise<T>` as a generic type**: Runtime `Promise` exists. The Checker treats many constructors as `Type::Function`, not `Promise<number>`.
- **Array and tuple generic types**: `T[]`, `Array<T>`, `ReadonlyArray<T>`. Arrays are JS values at runtime; there is no `Array<T>` in the lattice. Native fixed arrays are a different feature ([[specs/draconic/language/native-types/contract]]).
- **Built-in object types**: `Date`, `RegExp`, `Map<K,V>`, `Set<T>`, typed arrays, `Error` subclasses as distinct Checker types. Globals exist at runtime; check-time they are not a TS `lib.dom` / `lib.esnext` graph.

## Type-level programming TypeScript has

The “types as a language” layer. Draconic generics are identity-and-alias instantiation, not a type interpreter.

- **Generic constraints**: `T extends Foo`. No `extends` on type parameters.
- **Default type parameters**: `T = string`.
- **`const` type parameters**: TypeScript 5.0 `const T`.
- **Variance annotations**: `in` / `out` on type parameters.
- **Higher-kinded patterns and infer chains**: Common in TS libraries (`T extends (infer U)[] ? U : T`).
- **Overload signatures**: Multiple call signatures, implementation signature hiding, overload resolution order. One annotated `FnSig` per function.
- **Generic inference from context (full tsc)**: Contextual typing from the expected type of a callback argument, bidirectional inference, `NoInfer<T>`. Draconic infers generic function type arguments from actual arguments at the call (T04). That is not tsc’s inference engine.
- **Excess property checking (full)**: Draconic has T07.05 for a fresh literal vs an annotated shape. TypeScript also does weak-type detection, spread freshness, and intersection excess rules. Do not assume those extras.
- **Control-flow narrowing (full)**: TypeScript narrows on truthiness, equality, `in`, `instanceof`, discriminated unions, assertion functions, assignment, and `switch`. Draconic promises `typeof` narrowing. Other CF narrowing is not a locked contract.
- **Strictness flags as a family**: `strict`, `strictNullChecks`, `strictFunctionTypes`, `noImplicitAny`, `noImplicitThis`, `strictBindCallApply`, `useUnknownInCatchVariables`, `noUncheckedIndexedAccess`, `exactOptionalPropertyTypes`, `noImplicitReturns`, `noFallthroughCasesInSwitch`. Draconic has no `tsconfig`. Annotated vs unannotated is the honesty switch, not a flag matrix.

## Classes and object-oriented TypeScript

Runtime classes exist on the ECMA path (**E05**). TypeScript’s class *type* layer is mostly absent.

- **`implements`**: `class C implements I`. Not a Checker feature.
- **`abstract` classes and methods**.
- **Visibility as types**: `private`, `protected`, `public` (TypeScript fields, not ECMA `#` private). ECMA private fields `#x` exist as runtime; they are not TS visibility.
- **Parameter properties** (again): `constructor(private x: T)`.
- **Definite assignment assertions**: `x!: T`.
- **`override` modifier**.
- **Generic classes**: `class Box<T>`. Generics are aliases and functions (T04), not generic class types.
- **Generic methods with their own type params** beyond function generics.
- **Typed `this`**: `this: T` in methods, polymorphic `this` return.
- **Declaration merging of interfaces with classes**.
- **Mixins as a typed pattern** (`T extends Constructor`).
- **JSX element types**: `JSX.IntrinsicElements`, function components as types, `React.FC`. Draconic is not a JSX language. There is no `.tsx` product.

## Modules, declarations, and packages

TypeScript’s compilation unit is a project of `.ts` / `.d.ts` files plus Node resolution. Draconic’s unit is a Program; ESM linking is the Linker ([[architecture-linker]]). Packages are Go-style git modules ([[0009-go-style-git-packages]]), not npm.

- **`.d.ts` declaration files**: Authoring and consuming type-only packages. Out of scope ([[0005-ts-inspired-not-tsc]]).
- **Declaration emit**: `tsc --declaration` / `.d.ts` + `.d.ts.map`. JS backend emits JavaScript only.
- **DefinitelyTyped / `@types/*`**: The npm type graph for untyped JS. Not a goal; not npm-registry packages (draconic-language `pkg-git-modules` / product rule: no npm-registry packages).
- **`types` / `typings` in package.json**, `exports` types conditions.
- **Path mapping**: `compilerOptions.paths`, `baseUrl`.
- **Module resolution algorithms**: `node10`, `node16`, `nodenext`, `bundler`, `classic`. Linker loads a static ESM graph from an entry path; it is not tsc moduleResolution.
- **`allowJs` / `checkJs` / `// @ts-check`**.
- **`skipLibCheck`**, `typeRoots`, `types` array (which `@types` to include).
- **Project references**: Composite projects, `composite: true`, solution-style `tsconfig`.
- **`isolatedModules` / `verbatimModuleSyntax`**: Constraints for single-file transpilers. Draconic links then checks; different pipeline.
- **`import type` / `export type` as a full TS mode**: Parser has type-only import AST. That is not verbatimModuleSyntax, elision policy matching tsc, or type-only re-export as a package contract.
- **CommonJS interop flags**: `esModuleInterop`, `allowSyntheticDefaultImports`, `strictNullChecks` interaction with `export =`.
- **`export =` / `import = require()`**: TypeScript/CJS module shape. Draconic is ESM.
- **AMD / UMD / System module emit**. JS emit is JavaScript source, not tsc `--module`.
- **`resolveJsonModule` as a tsc flag**: JSON modules exist on the Linker path as ESM JSON; that is not the tsc flag set.

## Compiler product TypeScript has

tsc as a CLI and project tool. Draconic’s CLI is `draconic` ([[architecture-cli]], [[guides-toolchain]]).

- **`tsconfig.json`**: The whole option surface (hundreds of flags). There is no tsconfig.
- **Incremental build**: `.tsbuildinfo`.
- **`--watch` as tsc**: Draconic has `build --watch` / `check --watch` (U10) on its own pipeline, not tsc watch + incremental graph.
- **Emit today vs check tomorrow**: `noEmit`, `emitDeclarationOnly`, `noEmitOnError`, `pretty`, `listFiles`, `extendedDiagnostics`, `generateTrace`.
- **Target/lib matrix**: `target: ES5` … `ESNext`, `lib: ["dom", "es2023", …]`. Downlevel emit (e.g. async to generators for ES5) is a tsc product. Draconic JS emit is not “compile TS to older JS.” Native emit is LLVM.
- **JSX emit modes**: `react`, `react-jsx`, `preserve`, `react-native`.
- **Source map flags matching tsc**: Draconic has JS source maps (U03). That is not the tsc `inlineSources` / `mapRoot` matrix.
- **`tsc` language service host protocol**: The TypeScript language service used by VS Code, with complete, rename, code actions, organize imports, refactors, inlay hints, and project-wide find-all-references.
- **Compatibility promise**: Compiling existing TypeScript projects. Explicitly refused.

## Editors and developer experience

TypeScript’s real product is often the editor, not the type theory.

- **VS Code TypeScript extension**: Default language support, workspace versions of tsc, automatic import, quick fixes. Draconic ships no VS Code extension ([[architecture-editors]]).
- **Full LSP server**: `draconic-lsp` is an analysis library: diagnostics, hover types, go-to-definition over `check_source` (Script). It is not a `tower-lsp` process, not JSON-RPC, not multi-file Modules, not package imports, not pull diagnostics from a linked graph ([[architecture-lsp]], U06). One diagnostic on failed check, not a list.
- **Zed today**: Syntax highlighting for `.drac` via tree-sitter TypeScript plus native type names and `extern`. It does not start the analysis library. No Neovim or other editor package in-tree.
- **Completions, rename, code actions, organize imports, extract function, inlay hints**: Not in the analysis API.
- **Project-wide indexing**: TypeScript server loads a program. Draconic analysis is one source string through Frontend check.
- **Playground**: typescriptlang.org playground with shareable URLs. Public Learn/Reference is a site ([[0013-public-site-tanstack-start]]); in-page runners are later.
- **`d.ts` hover from node_modules**: The DefinitelyTyped experience. No npm types.

Related toolchain that *does* exist and is not tsc: `draconic check`, `draconic fmt`, `draconic test`, `draconic doc`, REPL, error codes, watch, coverage, shebang `run` (Roadmap **U**). Those are Draconic DX, not a TypeScript-shaped language service.

## Deliberate non-goals (not gaps to close)

Treat these as closed unless a human re-opens them with an ADR and a Roadmap row.

- **tsc compatibility**: Do not compile existing TypeScript projects or match flags ([[0005-ts-inspired-not-tsc]], `language.types:forbid-tsc-compatibility`).
- **TypeScript emit**: JS backend emits JavaScript ([[architecture-check]] trade-off).
- **npm-registry packages and `@types`**: Packages are git modules ([[0009-go-style-git-packages]]).
- **Self-hosting the Compiler in Draconic**.
- **Silent type erasure of native types**: Dual worlds need a real Checker, not tsc erasure.
- **Untyped JS staying permissive**: Not a defect. Annotations opt in.

Filing “needs tsc” or “drop-in TypeScript migration” is refused by the roadmap-audit bar.

## What Draconic has that TypeScript does not

Useful contrast so the catalog is not read as “behind TypeScript on every axis.”

- **Unboxed Native types**: `i8`–`i64`, `u8`–`u64`, `f32`, `f64`, native `bool`, structs, fixed arrays, pointers ([[0003-gc-runtime-and-dual-worlds]]).
- **Dual worlds**: JS values and native types in one Program with explicit `as` ([[specs/draconic/language/dual-worlds/contract]]).
- **Two backends from one IR**: JavaScript and LLVM ([[0002-shared-ir-dual-backends]]).
- **Full ECMA-262 goal including Embed**: `eval` / `new Function` on native via Embed ([[0004-full-ecma-262-and-embed]]).
- **Host I/O and git packages as language product**: Not “types for Node,” a different stdlib and package story.

TypeScript can describe some of those with `bigint`, branded numbers, or WASM types in comments. It cannot emit unboxed `i32` next to a JS object in one Program with a hard dual-world boundary.

## Examples

TypeScript (not Draconic):

```ts
interface User {
  id: string;
  name?: string;
  readonly email: string;
}

type Keys = keyof User;
type Id = User["id"];
type Flags = { [K in Keys]: boolean };
type Unpack<T> = T extends Promise<infer U> ? U : T;

function take(x: unknown): asserts x is User {
  // …
}

const u = { id: "1", email: "a@b.c", extra: true } as const;
take(u satisfies User);
```

None of `interface`, optional/readonly fields, `keyof`, indexed access, mapped types, `infer`, assertion functions, `as const`, or `satisfies` are Checker product.

Draconic (locked subset):

```js
type Point = { x: number; y: number };
type Box<T> = { value: T };

function id<T>(x: T): T {
  return x;
}

let p: Point = { x: 1, y: 2 };
let n: number | string = "hi";
if (typeof n === "string") {
  // n narrowed to string on this branch
}

let k: i32 = 1 as i32;
```

Shapes, aliases, generics, unions, `typeof` narrowing, and dual-world `as` into `i32` are the intended surface.

TypeScript assertion (not the Draconic rule):

```ts
const x = "hello" as number; // tsc: lie, still a string at runtime
```

Draconic `as` is a Dual-world conversion or a reject, not a lie.

## Reference

- **Decision**: [[0005-ts-inspired-not-tsc]]
- **Checker architecture**: [[architecture-check]]
- **Types contract and tests**: [[specs/draconic/language/types/contract]], [[specs/draconic/language/types/test]]
- **Dual worlds**: [[architecture-dual-worlds]], [[specs/draconic/language/dual-worlds/contract]]
- **Editors / LSP**: [[architecture-editors]], [[architecture-lsp]]
- **Packages**: [[0009-go-style-git-packages]], [[architecture-pkg]]
- **Completeness**: [[ROADMAP]] track **T** (done), [[overview-completeness]]
- **Glossary**: [[CONTEXT]]
