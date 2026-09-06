---
id: "architecture-pipeline"
title: "Compile pipeline"
kind: architecture
description: "How a Draconic Program becomes JS text or a native binary, who owns Script versus Module policy, and how diagnostics flow."
domain: draconic
area: architecture
tags: [architecture, compile-pipeline, toolchain]
created_at: "2026-09-06"
updated_at: "2026-09-06"
---

# Compile pipeline

## Overview

The Compiler turns a [[CONTEXT|Program]] into an artifact: ECMAScript source, or a native binary linked with the [[architecture-runtime|Runtime]]. One [[architecture-frontend|Frontend]] owns parse-or-link, Script versus Module goal, check, and lower into shared [[architecture-ir|IR]]. After that, callers choose the [[architecture-backend-js|JS backend]] or the [[architecture-backend-llvm|LLVM backend]]. Native-only and JS-only features must hard-error with a diagnostic. They must never emit silent wrong code.

The locked shape is [[0002-shared-ir-dual-backends]]: both backends consume the same IR. The host Compiler stays Rust forever ([[0001-rust-host-compiler]]); self-hosting is out of scope. Completeness is tracked on [[ROADMAP]].

## Context

Callers used to be able to wire parser, checker, and IR crates themselves, or guess Module from source substrings. That drifts Script versus Module semantics and splits link policy across the [[architecture-cli|CLI]], [[architecture-embed|Embed]], and tests.

The Frontend crate (`draconic-frontend`) is the seam. String inputs use `compile_source` (Script) or `compile_source_module` (Module goal, no filesystem graph). Filesystem entries use `compile_path`, which may call `link_entry` in [[architecture-linker]]. Relative imports are not resolved on string compile. Embed `eval` / `new Function` on native is the destination in [[0004-full-ecma-262-and-embed]]; today Embed compiles Script strings through that same Frontend.

## Design

Stages, in order:

- **Source or path**: a string Program, or a filesystem entry. The CLI also runs package lock ensure before path compile.
- **Parse or link**: `parse` / `parse_module`, or `link_entry` when the entry has ESM import/export syntax.
- **Check**: `check` (Script) or `check_module` (Module). Bind then typecheck. See [[architecture-check]].
- **Lower IR**: `lower` produces one `Module` IR unit.
- **Emit**: `emit_js` writes JS text, or `emit_llvm_ir_with_debug` plus `build_native_binary_with_lto` produces a native binary with the Runtime.

Frontend policy (not a source-text heuristic):

- **`compile_source` / `check_source`**: Script parse and Script check. Top-level `await` is rejected. Relative imports are not resolved. Embed and single-buffer inputs use this.
- **`compile_source_module` / `check_source_module`**: Module-goal parse and check (top-level `await` allowed). Still no link graph.
- **`compile_path` / `check_path`**: read the file, Script-first parse, retry Module when Script parse fails (export plus top-level `await` is the documented case). If the AST has import/export statements, call `link_entry` and check as Module; otherwise check as Script.

The CLI compile surface is `build --target js|native` and `run` (default target `js`). `check` stops after Frontend check. `parse`, `fmt`, `doc`, `extract`, `bindgen`, `mod` / `get`, and `test` are adjacent toolchain commands; they are not the full emit pipeline. Details live in [[system-design-compile-pipeline]].

## Trade-offs

- **Rust host Compiler forever**: durable LLVM, one language across Frontend, IR, and backends. Sacrifices a self-hosted Compiler ([[0001-rust-host-compiler]]).
- **Shared IR, dual backends**: semantics stay in one lowerer. Each backend must still refuse features it cannot express, instead of forking a typed AST ([[0002-shared-ir-dual-backends]]).
- **Frontend owns Script versus Module**: callers do not reassemble stage crates. String compile cannot resolve relative imports; filesystem entries pay for the linker.
- **Native Embed for eval**: destination is compiler-in-runtime so `eval` and `new Function` are not JS-backend-only ([[0004-full-ecma-262-and-embed]]). Today the Embed crate compiles and interprets a Script subset; the LLVM path folds a supported eval/Function subset at emit time.

## Consequences

- **Callers use Frontend**: CLI `build` / `run` / `check` and Embed `eval_source` go through `compile_path` or `compile_source`. They must not gate modules on source substrings.
- **Hard errors, not stubs**: JS emit rejects native pointers, `extern "C"`, and host APIs marked unavailable on js. LLVM emit returns a diagnostic when no lowering exists for the IR (empty programs are a documented hello demo, not a silent success for arbitrary IR).
- **Diagnostics are one type**: `Diagnostic` from `draconic-diagnostics` (message, span, optional code and help). Stages return `Result<_, Diagnostic>`. The CLI prints `error: {diagnostic}` and exits 1.
- **CLI is a surface, not the policy owner**: watch loops, permission flags on `run`, package ensure, and extra native link flags wrap the pipeline. They do not choose Script versus Module.
- **Sibling notes**: [[architecture-frontend]], [[architecture-ir]], [[architecture-backend-js]], [[architecture-backend-llvm]], [[architecture-runtime]], [[architecture-embed]], [[architecture-cli]], [[architecture-linker]], [[architecture-check]].
