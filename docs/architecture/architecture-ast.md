---
id: "architecture-ast"
title: "Frontend AST"
kind: architecture
description: "Program node categories, stable dump (B02), and deterministic printer (U05)."
domain: draconic
area: frontend
tags: [architecture, frontend, ast]
created_at: "2026-09-06"
updated_at: "2026-09-06"
---

# Frontend AST

## Overview

`draconic-ast` is the typed tree for a parsed [[CONTEXT|Program]]. Nodes carry [[architecture-diagnostics|Span]]. `dump_program` is the stable, indentation-based dump for snapshots and `draconic parse` (Roadmap B02, done). `print_program` is the deterministic source printer for `draconic fmt` (U05, done). The crate does not parse or typecheck.

## Context

[[architecture-parser]] builds this tree. [[architecture-check]] walks it. [[architecture-ir]] lowers from a checked Program, not from this dump. Parenthesized expressions are preserved (`Expr::Paren`) so dumps match source grouping.

## Design

### Crate layout

- [x] **src/lib.rs**: `Program` and all node types, `dump_program`, dump helpers, dump unit test
- [x] **src/print.rs**: `print_program` (U05), printer unit tests
- [x] **Cargo.toml**: `draconic-diagnostics`, `draconic-lexer` (`JsString` re-exported)

### Root

`Program { body: Vec<Stmt>, span }`.

### Bindings and patterns

- **BindingKind**: `Let` `Const` `Var` `Function` `Using` `AwaitUsing`. `is_lexical` / `is_const_like` helpers.
- **BindingPattern**: `Ident`, array, object, or assignment-only `Member`.
- **ArrayPatternElement**: `Elision`, `Pattern` (optional default), `Rest`.
- **ObjectPatternProp**: `Prop` (key, binding, shorthand, default) or `Rest`.
- **Param**: binding, optional `TypeAnn`, optional default, `rest`.

### Statements

`Expression`, `Let` (all `BindingKind`s, optional type annotation and init), `Empty`, `Block`, `If`, `While`, `DoWhile`, `For`, `ForIn`, `ForOf` (`is_await`), `Break` / `Continue` (optional label), `Labeled`, `Switch` (`SwitchCase`), `FunctionDeclaration` (type params, return type, async, generator), `ClassDeclaration`, `Return`, `Throw`, `Try` (optional catch param/body, optional finally), `With`, `ImportDeclaration`, `ExportNamedDeclaration`, `ExportDefaultDeclaration`, `ExportAllDeclaration`, `TypeAlias`, `ExternFunctionDeclaration`.

Module extras on import/export: specifiers, namespace, `ImportAttribute` (`with`/`assert`), `ImportPhase` (`Evaluation` `Defer` `Source`), `type_only`. Default import is a specifier with imported name `default`.

### Classes

`ClassElement`: `Constructor`, `Method` (static/async/generator/private/computed), `Accessor` (`AccessorKind` Get/Set), `Field`, `StaticBlock`. Keys are `ObjectKey` (`Ident` `String` `Computed`).

### Expressions

Literals: `Ident` `Number` `BigInt` `String` `RegExp` `Boolean` `Null` plus `TemplateLiteral` / `TaggedTemplate` (`TemplateElement` cooked quasis).

Atoms: `This` `Super` `NewTarget` `ImportMeta` `ImportCall` (phase + optional attributes argument).

Operators: `Unary` (`UnaryOp` includes `Await` `Yield` `YieldStar` `Ref` `&` `Deref` `*`), `Binary`, `Conditional`, `Assign` (`AssignOp` including logical/nullish assign), `Update` (`++` `--`).

Calls and members: `Call` (optional chaining, spread `Arg`), `New`, `MemberExpression` (computed, optional, private), `PrivateIn` (`#name in object`).

Functions and objects: `FunctionExpression`, `ClassExpression`, `ArrowFunction` (`ArrowBody` expr or block), `ObjectExpression` (`ObjectProp` property/accessor/spread), `ArrayExpression` (spread, elision, `trailing_comma`).

Other: `Paren` (dump fidelity), `As` (`expr as T`, T06, erased at emit), `ArrayPattern` / `ObjectPattern` as assignment targets.

### Types (syntax only)

`TypeAnn`: `Named`, `GenericApp`, `Object` (`TypeProp`), `Tuple`, `Pointer` (`*T`), `Union`, `Intersection`. `TypeParam` is a name on functions and aliases. The Checker assigns meaning; this crate does not.

### Dump stability (B02)

`dump_program` writes `Program\n` then indented children (two spaces per level). Shape is locked by:

- AST unit test `dump_let_number` (hand-built tree).
- Parser `parse_and_dump` snapshot tests (parse → dump equality).

`dump_ast` in the parser crate is this function. Changing dump text is a snapshot break. Spans are not printed.

### Printer (U05)

`print_program`: 2-space indent, stable spacing, no comment preservation. Parse → print → parse → print is idempotent for well-formed programs. Object/function/class expression statements are parenthesized to avoid ASI/declaration ambiguity.

### Tests

- **lib.rs**: dump snapshot for `let x = 1`.
- **print.rs**: printer snapshots for simple lets and related forms.

No `tests/` directory.

## Trade-offs

- **Paren nodes**: extra variants so dumps are not α-equivalent up to grouping.
- **Decorators absent**: parser discards `@` lists; no `Decorator` node.
- **Fmt drops comments**: U05 v1; trivia never entered the tree.

## Consequences

[[architecture-check]] and [[architecture-ir]] must match these variants exhaustively. Do not grow dump format without updating parser snapshots. Frontend callers should not pretty-print via `Debug`; use `dump_program` or `print_program`.
