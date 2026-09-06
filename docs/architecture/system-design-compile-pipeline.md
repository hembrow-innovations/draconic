---
id: "system-design-compile-pipeline"
title: "Compile pipeline system design"
kind: system-design
description: "Components, interfaces, and diagnostic flow from Program source to JS text or a native binary."
status: draft
domain: draconic
area: architecture
tags: [architecture, compile-pipeline, toolchain]
created_at: "2026-09-06"
updated_at: "2026-09-06"
---

# Compile pipeline system design

## Overview

This design covers the compile pipeline: how a [[CONTEXT|Program]] becomes JS text or a native binary. The [[architecture-frontend|Frontend]] owns Script versus Module (link) policy, then [[architecture-check|check]] and lower into shared [[architecture-ir|IR]]. The [[architecture-backend-js|JS backend]] and [[architecture-backend-llvm|LLVM backend]] consume that IR ([[0002-shared-ir-dual-backends]]). The [[architecture-cli|CLI]] and [[architecture-embed|Embed]] are callers. They must not wire parser, checker, or IR crates, and must not gate modules on source substrings.

See [[architecture-pipeline]] for the locked architecture. Completeness is [[ROADMAP]].

## Components

- **Parser (`draconic-parser`)**: `parse` is Script goal. `parse_module` is Module goal (strict; top-level `await` grammar). `parse_and_dump` dumps a Script AST for the CLI `parse` command.
- **Linker (`draconic-linker`)**: `link_entry` loads an entry path’s static ESM graph, mangles bindings, and flattens to one Program. Optional package context from `draconic.lock` plus module cache. Owned by [[architecture-linker]]; Frontend decides parse versus link.
- **Checker (`draconic-check`)**: `check` / `bind` are Script (top-level `await` rejected). `check_module` / `bind_module` are Module (top-level `await` allowed; top-level functions lexical). `check_for_target` / `check_module_for_target` add host-API availability for `CompileTarget::Js` or `Native`. Frontend path compile does not pass a target; JS host-API refusal also happens at emit. See [[architecture-check]].
- **Frontend (`draconic-frontend`)**: public compile and check entry points. Script-first detection on paths. Re-exports `CheckedProgram` and IR `Module`.
- **IR (`draconic-ir`)**: `lower(&CheckedProgram) -> Module`. Shared unit both backends consume ([[architecture-ir]]).
- **JS backend (`draconic-backend-js`)**: `emit_js` returns ECMAScript text. Native pointers, pointer operators, `extern "C"` (`has_extern_ffi`), and host APIs unavailable on js hard-error. Native scalars may polyfill or erase. `emit_js_with_map` exists; CLI `build` uses `emit_js` only.
- **LLVM backend (`draconic-backend-llvm`)**: `emit_llvm_ir` / `emit_llvm_ir_with_debug` select a supported subset adapter. Unmatched IR returns a diagnostic (no silent hello for non-empty unsupported programs). Empty programs emit a Runtime hello demo. `build_native_binary_with_lto` compiles LLVM IR plus Runtime C via clang. See [[architecture-backend-llvm]] and [[architecture-runtime]].
- **Embed (`draconic-embed`)**: `eval_source` compiles a Script string with `compile_source`, then interprets a subset. `eval_function_call` covers `Function` bodies. `fold_eval_program` folds eval/Function IR at emit for the native subset. Destination is compiler-in-runtime ([[0004-full-ecma-262-and-embed]]). See [[architecture-embed]].
- **Diagnostics (`draconic-diagnostics`)**: `Diagnostic` is the error type across stages.
- **CLI (`draconic-cli`)**: user surface. Compile is `build` / `run` via `compile_path`. Not the policy owner.

## Interfaces

Frontend (`draconic-frontend`):

- **`compile_source(source)`**: Script parse, `check`, `lower`. No filesystem link. Relative imports unresolved. Top-level `await` rejected.
- **`compile_source_module(source)`**: `parse_module`, `check_module`, `lower`. Top-level `await` allowed. Still no link graph.
- **`compile_path(entry)`**: `check_path` then `lower`.
- **`check_source(source)`**: `parse` then `check`. No lower.
- **`check_source_module(source)`**: `parse_module` then `check_module`. No lower.
- **`check_path(entry)`**: load Program (parse or `link_entry`), then `check` or `check_module`. No lower.

Path load (private `load_program`):

- **Read file**: I/O failure becomes `Diagnostic` with a dummy span.
- **Script-first**: try `parse`. If the Program body has import/export statements, `link_entry` and Module goal.
- **Module retry**: if Script parse fails, try `parse_module`. If that Program has module syntax, `link_entry` and Module goal; otherwise return the Script parse error. Export plus top-level `await` fails Script parse once `await` is IdentifierReference outside Module, then retries Module.

Detection is AST match on `ImportDeclaration`, `ExportNamedDeclaration`, `ExportDefaultDeclaration`, `ExportAllDeclaration` — not a source substring.

CLI compile (`build_program` in [[architecture-cli]]):

- **Pin**: toolchain pin from nearest `draconic.toml` before work (version/help skip).
- **Packages**: `ensure_locked_for_entry` before link/compile (`--offline` on `build` is cache-only).
- **IR**: `compile_path(input)`.
- **JS**: `emit_js`, write `{stem}.js` by default.
- **Native**: read source for `SourceDebug`, `emit_llvm_ir_with_debug`, `build_native_binary_with_lto`. Default output is the input stem. `--strip` / `--lto` / `--link <lib.a>` are native-only.

CLI surfaces that exist in `main` (do not invent flags):

- **`parse <file>`**: `parse_and_dump` (Script AST dump). Not Frontend.
- **`check [--watch] <file>`**: `check_path`. No `--target`.
- **`build --target js|native`**: optional `--watch`, `--offline`, `--strip`, `--lto`, `--link <lib.a>`, `-o` / `--out` / `--output`.
- **`run [--target js|native]`**: default target `js`. Builds to a temp artifact and execs (Node for js, the binary for native). Remaining args after the file are program argv. `--allow-fs-read` / `--allow-fs-write` / `--allow-net-listen` / `--allow-net-connect` set `DRACONIC_PERMISSIONS` (R02.03). Bare path or shebang invokes `run`.
- **`repl [--target js|embed]`**: js uses `compile_source` then Node; embed uses `eval_source`. Multi-line when parse looks like EOF; Script then Module parse for completeness.
- **`fmt [--check] <file>`**: Script-first then Module parse, `print_program`, no link.
- **`doc [--format md|html] [-o <out>] <file>`**: `/**` comments to markdown or HTML.
- **`extract <file>`**: v1 JSON via `parse_module` only.
- **`bindgen <header> [-o <out>]`**: C header to `extern "C"` module.
- **`get` / `mod tidy`**: git packages (not emit).
- **`test [--coverage] [--jobs <n>] <path>`**: conformance fixtures.
- **`version` / `help`**.

Watch (`check` and `build`): poll mtime (`DRACONIC_WATCH_POLL_MS`, default 200ms). Errors print; the loop continues.

## Data model

No separate schema note. IR shape is [[architecture-ir]].

- **Program**: AST unit (`draconic-ast`). Parser or linker output. Input to check.
- **CheckedProgram**: bound Program plus types, shapes, unions, intersections. Input to `lower`.
- **IR Module**: `locals`, `body`, `body_spans`, `shapes`, `has_extern_ffi`. Both backends take `&Module`.
- **Diagnostic**: `message`, `span` (`BytePos` half-open), optional `ErrorCode` (`E0300`…), optional `help`. Display is `message at start..end`, or `[E0xxx] message at start..end`. Pretty print with caret is available; CLI compile errors use `Display`.
- **CompileTarget**: `Js` or `Native` in the checker host-API registry. CLI `build`/`run` use a parallel `Target` enum (`js` | `native`). REPL adds `embed`.
- **EmbedValue**: undefined, null, boolean, number, string — completion values from Embed eval.

## Interactions

- **String Script (Embed, REPL js)**: source → `compile_source` → `parse` → `check` → `lower` → IR. Embed interprets. REPL js calls `emit_js` (last expression printed) and Node.
- **String Module**: source → `compile_source_module` → `parse_module` → `check_module` → `lower`. Callers that need a link graph must use a path.
- **Filesystem Script**: path → Script parse, no module syntax → `check` → `lower` → backend. Linker skipped.
- **Filesystem Module**: path → Script-first or Module retry → module syntax → `link_entry` → `check_module` → `lower` → backend. Linked entries allow top-level `await`.
- **JS emit**: `emit_js` rejects native-only IR, then writes text. CLI `run --target js` spawns `node` on the artifact.
- **Native emit**: `emit_llvm_ir_with_debug` then clang link with Runtime. `run --target native` execs the binary. Eval/Function on this path: Embed `fold_eval_program` for the supported subset, else unsupported-IR diagnostic.
- **Check-only**: `check_path`; no lower, no emit.
- **Adjacent, not emit**: `parse` dumps Script AST. `extract` walks a Module-goal AST to JSON. `fmt` reprints. `doc` / `bindgen` / `get` / `mod` / `test` do not produce JS or a native binary.

## Error handling

- **Type**: every compile stage returns `Result<T, Diagnostic>` from `draconic-diagnostics`.
- **Propagation**: first diagnostic wins. No retry across backends. CLI prints `error: {d}` and exits 1. Usage mistakes exit 2. `run` forwards the child exit code when 1–255.
- **I/O**: read/write/create-dir failures become `Diagnostic` with `Span::dummy()`.
- **Script versus Module**: failed Script parse retries Module only on path load (and fmt/REPL completeness). If Module parse also fails, the Script error is returned.
- **Native-only / JS-only**: hard `Diagnostic`, never silent wrong code. JS: pointers, pointer ops, `extern "C"` (`codes::EXTERN_UNSUPPORTED` / E0401), host APIs (`codes::HOST_API_UNSUPPORTED` / E0400). Native: unmatched IR uses `unsupported_native_diagnostic` (dummy span). Checker also has `check_for_target`; Frontend `check_path` does not call it today — js host-API refusal is enforced at `emit_js`.
- **Embed limits**: source larger than 1 MiB, alloc budget, or time budget fail closed as diagnostics, not catchable JS exceptions.
- **Watch**: print the error and keep polling. No process abort on a failed rebuild.
- **Packages / pin**: lock ensure failures become diagnostics. Required toolchain pin mismatch exits 1 before compile.

## Trade-offs

- **Rust host Compiler forever**: [[0001-rust-host-compiler]]. One language across Frontend, IR, backends, Embed. Not self-hosting.
- **Shared IR after Frontend**: [[0002-shared-ir-dual-backends]]. Rejects per-backend typed AST forks and a foreign IR. LLVM still uses subset adapters; unsupported programs error instead of emitting a stub (except the empty-program hello demo).
- **Frontend owns link policy**: string compile cannot resolve relative imports. Path compile pays for `link_entry`. Callers must not reassemble stages.
- **Embed for eval**: [[0004-full-ecma-262-and-embed]] rejects JS-backend-only eval. Current code compiles Script strings in the Embed crate and folds a native eval subset at emit. Shipping a full Compiler inside every native binary is the destination, not what `build --target native` links today.
- **CLI `check` has no `--target`**: host-API target policy is not applied on check-only; it is applied at JS emit (and via `check_for_target` when a caller passes a target).
- **Rejected**: gating Module on source substrings; silent JS emit of pointers or `extern "C"`; treating empty-hello as success for arbitrary native IR.
