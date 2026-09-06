---
id: "architecture-ir"
title: "Shared IR"
kind: architecture
description: "Shared IR module both backends consume after the Frontend."
domain: draconic
area: backends
tags: []
created_at: "2026-09-06"
updated_at: "2026-09-06"
---

# Shared IR

## Overview

The crate `draconic-ir` is a single file, `lib.rs`. After the [[architecture-frontend|Frontend]] (parse, bind, check, and link when needed), `lower` turns a `CheckedProgram` into one `Module`. The [[architecture-backend-js|JS backend]] and [[architecture-backend-llvm|LLVM backend]] both consume that module. This is the IR named in [[CONTEXT]] and locked by [[0002-shared-ir-dual-backends]].

## Context

A typed AST per backend was rejected so semantics would not drift. A foreign IR (WASM-only and similar) was rejected so GC, [[architecture-embed|Embed]], and JS-faithful behavior stay under Draconic control. See [[system-design-dual-backends]] and [[0002-shared-ir-dual-backends]].

## Design

### Module

`Module` is the top-level IR unit:

- **locals**: every bound symbol from the checker, plus synthetics allocated during lower, each with `id` (`LocalId`, the checker's `SymbolId`), `name`, `ty` (`IrType`, the checker's `Type`), and `kind` (`BindingKind`).
- **body**: a list of `Stmt` nodes. Not SSA. Not bytecode. Nested statements and expressions.
- **body_spans**: one `Span` per top-level `body` entry, taken from the originating AST statement. Expanded lowerings (a class becomes several statements) reuse that span.
- **shapes**: structural object shapes referenced by `Type::Shape` (native layouts).
- **has_extern_ffi**: true when the Program declared `extern "C"`. The body then contains one or more `Stmt::ExternFunction` ABI decls. The JS backend must hard-error (F08.01).

`lower(&CheckedProgram) -> Module` is the only public lowering entry.

### Instruction and value model

IR is a typed statement/expression tree. Every `Expr` variant carries `ty`. `Expr::ty()` reads that field. Values are not virtual registers; they are nested expressions, locals, and assignment targets.

Statements (`Stmt`) cover:

- **Declare / DeclareArrayPattern / DeclareObjectPattern**: `let` / `const` / `var` (and related kinds) with optional init. Pattern heads used as `for-in` / `for-of` left-hand sides may have `init: None`.
- **AssignLeft**: assignment-pattern or member LHS of `for (… in/of …)` without a declaration keyword.
- **Expr / Block / If / While / DoWhile / For / ForIn / ForOf / Break / Continue / Labeled / Switch**: control and expression statements, including `for await`.
- **Function**: `async? function *? name(params) { body }`.
- **ExternFunction**: `extern "C" function` with ABI string (v1 `"C"`), linkage name, native scalar or pointer param types, and `ret: None` for C `void`. No body.
- **Return / Throw / Try / With**: completion, exceptions, and non-strict Object Environment.

Expressions (`Expr`) include locals, bare `IdentName` (with-chain names, not a static local), number/bigint/string/regexp/template/tagged-template literals, boolean, null, `this`, `new.target`, `import.meta`, `ImportCall` (including phase and options), unary/binary/conditional, assign, update (`++`/`--`), call, `new`, function/arrow/method values, `super`, object and array literals, and member reads (computed, optional chaining).

Supporting types:

- **Arg / ArrayElement**: expr or spread (arrays also have elision).
- **ObjectProp / ObjectPropKey**: property, accessor, or spread; static string key or computed key.
- **AssignTarget**: local, with-chain name, member, native pointer deref (`*ptr = …`), array pattern, object pattern.
- **UpdateTarget**: local, name, or member.
- **Pattern / Param**: ident, nested array/object, assignment member, optional default, rest flag.

### What lower does

`lower` copies checker symbols into `locals`, walks the bound program body, and records `has_extern_ffi` when it sees `ExternFunctionDeclaration`.

Notable expansions (still one shared IR, not backend-specific):

- **Class declarations** desugar to a builder IIFE assigned to the outer mutable class name, with an inner immutable const local so methods close over the class name (E19.57). Class expressions are the same builder returning the constructor. Private fields/methods/accessors become WeakMap/WeakSet/function locals in `LowerCtx`. Derived constructors get `this` / `super` temps and [[Construct]] return completion wrapping.
- **Type aliases** are erased (no runtime value).
- **Empty statements** produce no IR.
- **Import/export** panic if they reach `lower`; they must be linked first ([[architecture-frontend]] / Linker).
- **Synthetic temps** for private compound/update and some destructuring prefixes are hoisted as `var` declares so strict class methods can assign them.

`LowerCtx` is owned by one `lower` call. No process-global state.

### Types on IR

`IrType` is the checker's `Type`. Native scalars, pointers (`Type::Ptr`), layouts (`Type::Shape` into `module.shapes`), and JS types coexist on the same tree ([[0003-gc-runtime-and-dual-worlds]], Dual worlds in [[CONTEXT]]). Pointer operators and deref assign targets stay in IR so the JS backend can reject them instead of inventing a second tree.

## Trade-offs

- **One IR after check**: both backends see the same desugarings (classes, private brands, extern ABI types). Semantics cannot fork at the typed AST.
- **Tree, not SSA**: close to the Program surface so JS emit can print ECMAScript. LLVM adapters classify and lower subsets; there is no single SSA pipeline in this crate.
- **Synthetics in the same Module**: class/private desugar lives in IR so backends do not re-implement it.

## Consequences

- Callers run [[architecture-frontend]] then `lower`; they do not lower AST themselves.
- Portable vs native-only vs JS-only is not an IR enum. Policy is `has_extern_ffi`, pointer types/operators, host `IdentName`s, and each backend's hard-error path. See [[system-design-dual-backends]].
- `body_spans` feed JS source maps and LLVM DWARF. Nested statements share the enclosing top-level origin unless a backend adds finer markers.
- Pipeline neighbors: [[architecture-pipeline]], [[architecture-embed]].
