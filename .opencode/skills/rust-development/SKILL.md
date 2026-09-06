---
name: rust-development
description: Rust toolchain crates on Cargo, edition 2021, Diagnostic results, and the compiler pipeline. Use when building or reviewing crates, modules, APIs, errors, IR, backends, runtime, embed, workspace tests, file budget, or Rust bugs.
---

# Rust development

Production Rust for this toolchain. Folds codebase-design (crate modules) and diagnose (Rust bugs; keep the standalone skill for non-Rust). Detail lives in `rules/<id>.md`.

## Discover first

1. Crate first. Find the crate that owns the seam. Copy one neighboring module in that crate.
2. Product callers enter through `draconic-frontend` (`compile_source`, `compile_path`, `check_source`). Do not rewire parser → check → lower.
3. Compiler failures are `Diagnostic` plus `codes::*`. pkg uses focused enums.
4. File target ≤1000 LOC. Split by feature seam (`es_*`, `host_*`, pkg files).

Working notes go through **management**. Durable decisions go through **docs**. Vocabulary lives in `CONTEXT.md`.

## Stack

- **Tooling.** cargo only. edition 2021. Internal crates via `[workspace.dependencies]`.
- **Pipeline.** lexer → parser → ast → linker → check → ir → js|llvm. Frontend owns Script vs Module.
- **Errors.** `Result<T, Diagnostic>`. pkg hand-written `Error` enums.
- **Tests.** same-file `#[cfg(test)]`. Language bar is `tests/conformance`. `cargo test --workspace`.
- **Width.** Target ≤1000 LOC. `mod foo;` plus `foo.rs`. No `mod.rs` trees.

### Prefer

- **tool-**, **crate-**, **size-**, **arch-** cargo workspace, crate roles, file modules, deep seams
- **err-*** Diagnostic, codes, pkg enums, hard-error
- **pipe-**, **ir-**, **dual-*** frontend entry, shared IR, dual worlds
- **test-*** same-file units, workspace oracle, conformance, both targets
- **api-**, **type-**, **own-**, **vis-** concrete fns, newtypes, Box AST
- **backend-**, **host-**, **embed-**, **except-** emit seams, host registry, abort policy

### Apply carefully

- **unsafe-rare** CLI `isatty` only
- **dep-pinned-unused** thiserror, serde, rayon are pinned — leave them idle in product crates
- **own-thread-local** linker package context and LLVM JS interp only
- **diag-loop** for hard bugs. Build a tight red loop first
- New workspace members, standalone fuzz packages, `#[allow]`

### Do not introduce

- Python, Node, or Draconic as the compiler host
- anyhow, clap, tower-lsp, inkwell, llvm-sys, serde-for-toml
- A second AST, Diagnostic, or IR
- `cargo test --workspace --lib --bins` as a fake oracle
- Silent JS emit of native-only features
- Deno deny-by-default host I/O
- Process-global lower state or unmeasured `Cow`

## When to apply

- Building or reviewing a crate, module, or public fn
- Diagnostics, pkg errors, IR, backends, runtime, embed
- Workspace tests, conformance fixtures, file splits
- Ownership, newtypes, or Rust diagnosis

## Testing, review, audit

Do not fold these. Load the matching skill and stop:

- **spec** purpose, contract, test folders
- **tdd** red-green
- **draconic-loop** one Roadmap atom
- **gauntlet-loop** implement-then-critic
- **diagnose** hard bugs outside this pack's `diag-loop`

## Rule categories by priority

- **1 CRITICAL** Tooling + layout (`tool-`, `crate-`, `size-`, `arch-`)
- **2 CRITICAL** Errors (`err-`)
- **3 CRITICAL** Pipeline (`pipe-`, `ir-`, `dual-`)
- **4 CRITICAL** Tests (`test-`)
- **5 HIGH** Types + APIs (`api-`, `type-`, `own-`, `vis-`)
- **6 HIGH** Backends + runtime (`backend-`, `host-`, `embed-`, `except-`)
- **7 MEDIUM** Deps + rustc (`dep-`, `rustdoc-`, `unsafe-`, `btreemap-`)
- **8 LOW** Avoid + diag (`avoid-`, `diag-`)

## Quick reference

### 1. Tooling + layout (CRITICAL)

- `tool-cargo-edition` - cargo and edition 2021 only
- `tool-workspace-deps` - internal crates via workspace.dependencies
- `crate-members` - one crate per pipeline stage; copy neighbors
- `crate-frontend-entry` - callers use draconic-frontend
- `size-file-budget` - target ≤1000 LOC; split by feature seam
- `arch-file-modules` - `mod foo;` plus `foo.rs`; no mod.rs
- `arch-deep-modules` - small interface, lots of behaviour, real seams

### 2. Errors (CRITICAL)

- `err-diagnostic` `err-codes` `err-pkg-enum` `err-hard-error`

### 3. Pipeline (CRITICAL)

- `pipe-stages` `pipe-script-module` `ir-shared` `dual-worlds` `dual-js-policy`

### 4. Tests (CRITICAL)

- `test-same-file` `test-workspace` `test-conformance` `test-both-targets` `test-fuzz-hooks`

### 5. Types + APIs (HIGH)

- `api-borrow-own` `api-concrete` `type-enums` `type-newtypes` `own-box-ast` `vis-pub-crate`

### 6. Backends + runtime (HIGH)

- `backend-js-emit` `backend-llvm-text` `host-registry` `host-permissive` `embed-eval` `except-abort`

### 7. Deps + rustc (MEDIUM)

- `dep-no-extra` `dep-pinned-unused` `rustdoc-roadmap` `unsafe-rare` `own-no-global` `own-thread-local` `btreemap-lock`

### 8. Avoid / diag (LOW)

- `avoid-self-host` `avoid-second-ir` `avoid-narrow-oracle` `avoid-cow`
- `diag-loop` - tight red loop before hypothesising

## How to use

```text
rules/crate-frontend-entry.md
rules/err-diagnostic.md
rules/test-workspace.md
```

Pick 1–N rule ids for the task. `Read` only those `rules/<id>.md` files. Do not bulk-read `rules/` or load all of `AGENTS.md` unless asked or stuck. Prefer higher-priority categories when reviewing.

Each rule: why → incorrect → correct → notes.
