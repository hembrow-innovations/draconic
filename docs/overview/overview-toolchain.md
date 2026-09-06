---
id: overview-toolchain
title: Draconic toolchain
kind: overview
domain: draconic
area: overview
tags: [overview]
created_at: "2026-09-06"
updated_at: "2026-09-06"
description: Orientation to the Draconic language toolchain, compile pipeline, crates, and public site versus this vault.
---

# Draconic toolchain

## Overview

This repo **is** the Draconic Toolchain: Compiler, Runtime, Embed, and CLI. Draconic the language is a full ECMAScript superset with a TypeScript-inspired Checker and native systems types. The same Program can lower through a shared IR to a JS backend or an LLVM backend. Terms are defined in [[CONTEXT]]; this note does not repeat the glossary.

Find the rest of the hub in [[overview-vault]] and [[overview-completeness]]. Domain layout is [[domain]]. Completeness is [[ROADMAP]].

## Context

The product a developer runs is the Toolchain, not a typed-JavaScript-only compiler and not a separate FFI language. JS values and native types coexist as Dual worlds. Native binaries link a Runtime with tracing GC. On the native target, Embed ships enough Compiler to compile `eval` and `new Function` at run time.

Locked host and pipeline choices live in [[0001-rust-host-compiler]], [[0002-shared-ir-dual-backends]], [[0003-gc-runtime-and-dual-worlds]], [[0004-full-ecma-262-and-embed]], and [[0005-ts-inspired-not-tsc]]. Language job and fences are in [[purpose]]. How to operate the Toolchain is [[guides-toolchain]]. Public Learn and Reference are not this vault; see [[guides-public-docs]], [[0010-public-docs-draconic-ssg]], and [[0013-public-site-tanstack-start]].

## Design

### Language surface

- **ECMAScript superset**: full ECMA-262 destination, not a subset or a new syntax ([[0004-full-ecma-262-and-embed]]).
- **Checker**: TypeScript-inspired static types; not tsc-compatible; JS emit is JavaScript, not TypeScript ([[0005-ts-inspired-not-tsc]], [[architecture-check]]).
- **Native types**: unboxed systems types such as `i32`, `i64`, and fixed structs, outside the GC heap ([[architecture-dual-worlds]]).
- **Dual worlds**: JS values and native types in one Program at explicit type and lowering boundaries ([[architecture-dual-worlds]], [[0003-gc-runtime-and-dual-worlds]]).
- **JS backend**: IR to ECMAScript source ([[architecture-backend-js]]).
- **LLVM backend**: IR through LLVM to a native binary or object, linked with the Runtime ([[architecture-backend-llvm]], [[system-design-dual-backends]]).
- **Runtime GC**: tracing GC for JS values on native ([[architecture-runtime]], [[system-design-gc-runtime]]).
- **Embed**: Compiler (or equivalent) inside the Runtime for `eval` / `new Function` ([[architecture-embed]], [[api-embed]]).
- **CLI**: `draconic` parse, check, fmt, build, run, repl, test, get, mod tidy, bindgen ([[architecture-cli]], [[api-cli]]).

### Pipeline

Callers use Frontend; they do not wire stage crates. Script versus Module link policy lives in Frontend. The Linker loads an entry path’s ESM import graph when Frontend chooses link over a single-file parse.

- **lexer**: scan source to tokens ([[architecture-lexer]]).
- **parser / AST**: parse a Program; AST is the syntax tree ([[architecture-parser]], [[architecture-ast]]).
- **Frontend**: parse, bind, typecheck, lower to IR; Script vs Module (link) policy ([[architecture-frontend]], [[architecture-linker]]).
- **Checker**: assign and validate types ([[architecture-check]]).
- **shared IR**: both backends lower from this IR after Frontend ([[architecture-ir]], [[0002-shared-ir-dual-backends]]).
- **JS or LLVM**: JS backend or LLVM backend ([[architecture-backend-js]], [[architecture-backend-llvm]], [[system-design-dual-backends]]).

End-to-end shape is [[architecture-pipeline]] and [[system-design-compile-pipeline]]. Diagnostics span the pipeline ([[architecture-diagnostics]]).

### Crates

Workspace members under `crates/` (from the root Cargo workspace):

- **draconic-diagnostics**: spans, messages, pretty print, error codes.
- **draconic-ast**: AST.
- **draconic-lexer**: lexer.
- **draconic-parser**: parser.
- **draconic-linker**: ESM import graph, mangle, flatten to one Program.
- **draconic-check**: binder and Checker.
- **draconic-ir**: shared IR.
- **draconic-frontend**: Frontend entry; Script vs Module link policy.
- **draconic-backend-js**: JS backend.
- **draconic-backend-llvm**: LLVM backend.
- **draconic-runtime**: Runtime (GC, job queue, host hooks, Embed path).
- **draconic-embed**: Embed.
- **draconic-lsp**: language server ([[architecture-lsp]], [[architecture-editors]]).
- **draconic-pkg**: git-backed packages ([[architecture-pkg]], [[0009-go-style-git-packages]]).
- **draconic-cli**: `draconic` CLI ([[architecture-cli]], [[api-cli]]).

### Test crates

Workspace test members (not ad-hoc scripts):

- **tests/integration**: toolchain integration.
- **tests/conformance**: Conformance suite fixtures.
- **tests/packages**: package tests.
- **tests/test262**: staged Test262 harness ([[0007-test262-staged-roll-in]]).

Prefer `cargo test --workspace` and the `draconic` CLI. A Roadmap item is done only when tests are green on applicable targets; see [[overview-completeness]] and [[ROADMAP]].

### Public site versus this vault

Public Learn and Reference sources live in `website/`. Presentation is a TanStack Start app ([[0013-public-site-tanstack-start]], [[guides-public-docs]]). The vault-versus-site split stays ([[0010-public-docs-draconic-ssg]]). This `docs/` vault is agent and toolchain knowledge. Do not treat the vault as the public site.

## Trade-offs

Recorded, not re-litigated here: Rust host Compiler with no self-host ([[0001-rust-host-compiler]]); one shared IR rather than per-backend typed ASTs ([[0002-shared-ir-dual-backends]]); tracing GC plus unboxed native types ([[0003-gc-runtime-and-dual-worlds]]); full ECMA-262 including Embed rather than JS-only eval ([[0004-full-ecma-262-and-embed]]); Checker inspired by TypeScript without tsc compatibility ([[0005-ts-inspired-not-tsc]]).

## Related notes

Vault map and kind folders: [[overview-vault]]. Completeness and Loop: [[overview-completeness]]. Glossary: [[CONTEXT]]. Layout: [[domain]]. Language purpose: [[purpose]]. Vault standard: [[standards-docs-vault]].

- **architecture-pipeline**: [[architecture-pipeline]]
- **system-design-compile-pipeline**: [[system-design-compile-pipeline]]
- **architecture-frontend**: [[architecture-frontend]]
- **architecture-lexer**: [[architecture-lexer]]
- **architecture-parser**: [[architecture-parser]]
- **architecture-ast**: [[architecture-ast]]
- **architecture-check**: [[architecture-check]]
- **architecture-diagnostics**: [[architecture-diagnostics]]
- **architecture-ir**: [[architecture-ir]]
- **architecture-backend-js**: [[architecture-backend-js]]
- **architecture-backend-llvm**: [[architecture-backend-llvm]]
- **system-design-dual-backends**: [[system-design-dual-backends]]
- **architecture-runtime**: [[architecture-runtime]]
- **architecture-embed**: [[architecture-embed]]
- **architecture-dual-worlds**: [[architecture-dual-worlds]]
- **system-design-gc-runtime**: [[system-design-gc-runtime]]
- **architecture-cli**: [[architecture-cli]]
- **architecture-lsp**: [[architecture-lsp]]
- **architecture-pkg**: [[architecture-pkg]]
- **architecture-linker**: [[architecture-linker]]
- **architecture-editors**: [[architecture-editors]]
- **guides-toolchain**: [[guides-toolchain]]
- **guides-public-docs**: [[guides-public-docs]]
- **api-cli**: [[api-cli]]
- **api-embed**: [[api-embed]]
- **security**: [[security]]
- **performance**: [[performance]]
- **reliability**: [[reliability]]
