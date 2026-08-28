# Draconic

Draconic is a programming language: a full ECMAScript superset with TypeScript-inspired static types and native systems types, compiling to JavaScript and to native binaries via LLVM.

## Language product

**Draconic**:
The language itself — source programs, their meaning, and the surface syntax developers write.
_Avoid_: the compiler, “the project”, the repo name

**Program**:
A unit of Draconic source that the toolchain accepts as input (file or string).
_Avoid_: script, module (unless meaning ECMAScript Module), codebase

**Native type**:
A static, unboxed systems type (e.g. `i32`, `i64`, fixed structs) that is not a JavaScript language type.
_Avoid_: primitive (ambiguous with JS primitives), POD, rust type

**JS value**:
A heap-managed value with JavaScript semantics (objects, arrays, strings, closures, etc.).
_Avoid_: dynamic value, any, object (when meaning the whole heap universe)

**Dual worlds**:
The coexistence of JS values and native types in one program, with explicit boundaries at the type and lowering level.
_Avoid_: FFI-only model, “just typed JS”

## Toolchain

**Toolchain**:
The whole Draconic product a developer runs: compiler, runtime, embed, and CLI.
_Avoid_: compiler (when meaning the whole product), SDK

**Compiler**:
The Rust program that turns Draconic source into artifacts (JS text or native object/binary), not including the shipped runtime process itself.
_Avoid_: tsc, transpiler (when the native path is in view)

**Frontend**:
The compile path from source (or a filesystem entry) through parse, bind, typecheck, and lower into IR. Script vs Module (link) policy lives here; callers do not wire stage crates.
_Avoid_: parser (when meaning the whole front half)

**Linker**:
Loads an entry path’s ESM import graph, mangles bindings, and flattens to one Program. Owned by `draconic-linker`; Frontend chooses parse vs link. Not part of the Parser product.
_Avoid_: bundler (when meaning our static link step), parser

**IR**:
The shared intermediate representation both backends lower from after the Frontend.
_Avoid_: AST (post-check), bytecode (unless a concrete IR form is bytecode)

**JS backend**:
The lowering from IR to ECMAScript source (or equivalent) with semantic equivalence where the target can express the program.
_Avoid_: emitter, transpiler, tsc

**LLVM backend**:
The lowering from IR through LLVM to a native binary (or object), linked with the Runtime.
_Avoid_: native backend (prefer this only as informal shorthand), codegen

**Runtime**:
The native support linked into binaries: GC for JS values, async job queue, standard library hooks, and the Embed path.
_Avoid_: VM (unless a true bytecode VM is meant), libc

**Catchable exception**:
A JS-value failure a Program can handle with `try`/`catch` (user `throw`, ECMA Error objects). Does not abort the process. See ADR-0011 / Roadmap **R04.01**.
_Avoid_: panic, abort (those are process abort, **R04.02**)

**Embed**:
The compiler (or equivalent) shipped inside the Runtime so `eval`, `new Function`, and similar can compile source at run time on the native target.
_Avoid_: interpreter (unless that is the chosen eval strategy), JIT (unless tiered compilation is meant)

## Types and checking

**Checker**:
The static analysis pass that assigns and validates types (TypeScript-inspired, not tsc-compatible).
_Avoid_: typechecker as a product name, tsc

**Portable program**:
A Program that both backends can accept with equivalent observable behavior (after documented polyfills).
_Avoid_: isomorphic, universal

**Native-only** / **JS-only**:
A feature or Program that is valid on exactly one backend; the other must hard-error with a diagnostic, never silent wrong code.
_Avoid_: unsupported (too vague)

## Completeness process

**Roadmap**:
The ordered feature checklist that defines what remains for language completeness; source of truth with the test suite.
_Avoid_: backlog, kanban, tickets (for this loop)

**Conformance suite**:
Tests that pin ECMA-262 and native-type behavior; a Roadmap item is done only when its tests are green on the applicable targets.
_Avoid_: unit tests (when meaning the suite as truth), spec tests (ambiguous)

**Loop**:
One atomic iteration of the mega-loop skill: pick the next Roadmap item, red-green it, mark done only when tests pass.
_Avoid_: sprint, agent session (session may host one Loop)
