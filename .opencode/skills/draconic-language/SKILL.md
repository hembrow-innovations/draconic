---
name: draconic-language
description: Draconic language and toolchain on Rust. Use when implementing Frontend, IR, JS backend, LLVM backend, Runtime, Checker, Conformance fixtures, dual worlds, native types, host I/O, packages, CLI, or language semantics. When another skill needs toolchain conventions.
---

# Draconic language

Toolchain for the language. Folds compile-path seams, dual-world rules, Conformance layout, and CLI/host/package product law. Detail lives in `rules/<id>.md`.

## Discover first

1. Frontend first. Callers compile through `compile_path` / `check_path` / `compile_source`. Do not re-assemble parser, linker, checker, and IR.
2. Terms from `CONTEXT.md`. Program, Frontend, IR, Dual worlds, JS value, Native type, Runtime, Embed, Conformance suite.
3. Locked decisions in `docs/adr/`. The Roadmap Tests column is the Done bar.
4. Nested crate modules. File target ≤1000 LOC; hard 1250 for new files.

Loop work goes through **draconic-loop**. Durable decisions go through **docs**. Module seams go through **codebase-design**.

## Stack

- **Host.** Rust compiler permanently. Cargo workspace. `draconic` CLI.
- **Frontend.** Parse or link, then check, then lower to one IR.
- **Backends.** JS emit and LLVM emit both consume that IR.
- **Runtime.** Tracing GC for JS values. Native types stay unboxed.
- **Types.** TypeScript-inspired Checker. Not tsc.

### Prefer

- **tool-**, **size-**, **arch-** cargo, file budget, crate seams
- **pipe-*** Frontend entries, Script vs Module, linker
- **dual-*** `as` boundary, unboxed native, host globals
- **diag-*** hard-error, stable codes, real spans
- **ir-***, **js-***, **llvm-*** shared IR, N04 polyfill, adapter dispatch
- **test-*** Conformance fixtures, workspace oracle

### Apply carefully

- **rt-eval-budgets** Embed is a subset; grow it with fixtures
- **pipe-check-target** `check_path` is target-neutral; emit still rejects
- **lsp-analysis-only** hover and goto, not a new language server
- **diagnose** for hard bugs. Build a tight red loop first

### Do not introduce

- Self-hosting the Compiler in Draconic
- tsc compatibility, TypeScript emit, or npm-registry packages
- A second IR or a typed-AST fork per backend
- Deno deny-by-default permissions
- Silent subsetting (hello stub, erased pointers, wrong-backend success)
- Module detection via `source.contains("import")`
- Catchable JS exceptions for GC, OOM, or abort
- Invented Roadmap work when the board has no `todo`

## When to apply

- Compiling a Program or changing a crate on the compile path
- Dual worlds, native types, Checker, IR, either backend
- Conformance fixtures, Test262 allowlist, CLI, host I/O, packages

## Loop, tests, diagnosis

Do not fold these. Load the matching skill and stop:

- **draconic-loop** one Roadmap atom, test-first
- **tdd** red-green at a seam
- **gauntlet-loop** named implement-and-verify
- **diagnose** hard bugs
- **docs** / **domain-modeling** glossary and ADRs
- **codebase-design** depth and seams

## Rule categories by priority

- **1 CRITICAL** Tooling + layout (`tool-`, `size-`, `arch-`)
- **2 CRITICAL** Frontend (`pipe-`)
- **3 CRITICAL** Dual worlds (`dual-`)
- **4 CRITICAL** Hard-error (`diag-`, `js-n04-polyfill`, `llvm-no-hello-stub`)
- **5 HIGH** IR + backends (`ir-`, `js-`, `llvm-`)
- **6 HIGH** Runtime (`rt-`)
- **7 HIGH** Conformance (`test-`)
- **8 MEDIUM** CLI, host, packages (`cli-`, `host-`, `pkg-`)
- **9 MEDIUM** Product + vocab (`prod-`, `vocab-`, `lsp-`)

## Quick reference

### 1. Tooling + layout (CRITICAL)

- `tool-cargo-rust` - cargo and Rust only; no self-host
- `size-file-budget` - target ≤1000 LOC, hard 1250 for new files
- `size-target-dir` - keep `target/` under 10GB
- `arch-crate-seams` - enter at Frontend; do not skip crates
- `arch-deep-modules` - small interface, lots of behaviour

### 2. Frontend (CRITICAL)

- `pipe-compile-path` `pipe-script-module` `pipe-linker-not-parser` `pipe-embed-source` `pipe-check-target`

### 3. Dual worlds (CRITICAL)

- `dual-as-boundary` `dual-no-silent-widen` `dual-native-unboxed` `dual-host-globals`

### 4. Hard-error (CRITICAL)

- `diag-hard-error` `diag-codes` `diag-spans` `js-n04-polyfill` `llvm-no-hello-stub`

### 5–6. IR, backends, Runtime (HIGH)

- `ir-shared` `ir-after-link` `ir-as-erased` `llvm-adapter-dispatch` `js-emit-javascript` `js-polyfill-runtime`
- `rt-catchable-vs-abort` `rt-eval-budgets` `rt-gc-js-values`

### 7. Conformance (HIGH)

- `test-conformance-fixtures` `test-native-observations` `test-meta-sidecar` `test-workspace-oracle` `test-test262-staged`

### 8–9. CLI, host, product (MEDIUM)

- `cli-run-default-js` `cli-build-requires-target`
- `host-permissive-default` `host-sockets-first` `pkg-git-modules`
- `prod-not-tsc` `prod-full-ecma` `prod-rust-host` `vocab-context` `lsp-analysis-only`

## How to use

```text
rules/pipe-compile-path.md
rules/dual-as-boundary.md
rules/test-conformance-fixtures.md
```

Pick 1–N rule ids for the task. `Read` only those `rules/<id>.md` files. Do not bulk-read `rules/` or load all of `AGENTS.md` unless asked or stuck. Prefer higher-priority categories when reviewing.

Each rule: why → incorrect → correct → notes.
