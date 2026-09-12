---
id: "task-912-js-library-named-esm"
title: "Opt-in JS named ESM library emit"
kind: task
status: completed
mode: afk
blocked_by:
  - task-910-entry-export-table
sprint: "js-library-esm"
slice: "slice-911-js-library-named-esm"
tags: [ js, linker, distribution ]
created_at: "2026-09-12T11:05:00Z"
updated_at: "2026-09-12T22:20:00Z"
---
# Opt-in JS named ESM library emit

## Blocked by

[[task-910-entry-export-table]]: IR must already carry entry named-export names.

## Done

`draconic build --target js --library` writes an artifact Node can `import { name } from`. Slice O1–O3 hold. Promise `toolchain.cli:build-js-library-esm` is asserted and tested.

## Context

Current: JS emit is a script. Node `import { view } from` fails. `draconic run` spawns `node` on the artifact with no `--input-type=module`. That must keep working.

Desired: Opt-in `--library` on `draconic build --target js` appends named ESM exports from IR metadata (`export { local as publicName }` when names differ). A two-file re-export exposes the public name, not `__m0_*`. Native `--library` is rejected. Default build and run stay scripts.

Out of scope: default export, `export *`, always-ESM, unflattened multi-file emit, npm, copying emit in a consumer repo.

Always `draconic build … -o` into a temp path. Do not emit `{stem}.out.js` beside fixtures or examples.

## Verify

Slice O1–O3 CHECK print their EXPECT tokens and exit 0. `cargo test -p draconic-cli --offline` prints `test result: ok.`

scope: CLI `build --library` (js only), JS backend named-export wrapper, toolchain contract and test map for `toolchain.cli:build-js-library-esm`, CLI tests. Not LLVM emit. Not `draconic run` host change.

## Links

[[slice-911-js-library-named-esm]] [[task-910-entry-export-table]] [[ticket-908-js-library-esm-export]] [[js-library-esm]]

## Agent Brief

**Category:** enhancement
**Summary:** Opt-in `draconic build --target js --library` so Node can `import { name } from` flattened named exports.

**Drain:** `/afk-task`. Blocked until [[task-910-entry-export-table]] is `completed`. Claim only when unblocked.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Skills:** load **tdd**, **draconic-language**, **rust-development**. Compiler sitting, not a Roadmap atom.

**Intent (required when product behaviour changes):**
- This **is** product behaviour (new build mode). **No product behaviour change** is false.
- Promise ids: assert `toolchain.cli:build-js-library-esm`. Keep `toolchain.cli:build-targets`, `toolchain.cli:build-scratch-name`, and `toolchain.cli:run-execute`. Do not invent npm or package-resolve promises.
- Purpose: [[Toolchain purpose]]. Dual backends [[0002-shared-ir-dual-backends]]. Packages stay git modules [[0009-go-style-git-packages]] / [[Packages purpose]]. Distribution location [[location-224-distribution]] is not rewritten.
- Contract-first: edit [[Toolchain — Contract]] and [[Toolchain tests]] for the new promise, then lock with Node `import`, then implement. Do not treat `js.check` append as the oracle.

**Current behavior:**
`draconic build --target js` on `export const view = "view"` writes `const view = "view"` (no `export`). A two-file re-export writes a mangled local and no public export. Node `import { view } from` fails. `node` on the artifact as a script exits 0. CLI has `--target`, `-o`, `--watch`, `--offline`, native `--strip` / `--lto` / `--link`. No `--library`.

**Desired behavior:**
`draconic build --target js --library <entry> -o <temp>`:
- Single-file `export const view = "view"` → Node `import { view } from` that file yields `"view"`.
- Entry `export { view } from "./dep.drac"` → Node `import { view }` yields the dependency value. Public name is `view`, not `__m0_view`.
- `export { local as publicName }` exposes `publicName`.
- Help and usage list `--library`.
- `--library` with `--target native` is a parse/usage error, same class as native-only flags on js.
- Without `--library`, `draconic run --target js` of a Module that authors named exports still runs as a script and prints. Default scratch names unchanged.
- Empty named-export table: do not force `export {}` (that would SyntaxError the script host).

**Key interfaces:**
- CLI `BuildArgs` / `parse_build_args` / `cmd_build`. New opt-in `--library`. `--target` still required. No clap.
- JS `emit_js` (and the build path that calls it). In library mode, after the script body, emit named `export { local as publicName }` from IR metadata. Do not restore `Stmt::Export` through check.
- IR export metadata from [[task-910-entry-export-table]]. LLVM ignores it.
- Node verification must `import` the file (`--input-type=module` or `.mjs`). Do not lock emit shape with `js.check` alone.

**Acceptance criteria:**
- [x] Slice O1 CHECK prints `named-ok` and exits 0
- [x] Slice O2 CHECK prints `reexport-ok` and exits 0
- [x] Slice O3 CHECK prints `script-ok` and exits 0
- [x] `toolchain.cli:build-js-library-esm` is on [[Toolchain — Contract]] with a `test:` pointer, and [[Toolchain tests]] names that test
- [x] `cargo test -p draconic-cli --offline` prints `test result: ok.`
- [x] Promise ids listed above still hold (or were deliberately edited)

**Out of scope:**
- default export and `export *` (later children of [[ticket-908-js-library-esm-export]])
- Always emitting ESM; changing `draconic run` to `node --input-type=module`
- Keeping `export` statements through check and lower
- Multi-file ESM without flatten
- npm as v1 packages
- Consumer workarounds
- LLVM library emit
- `cargo test --workspace` as the unit oracle

## Gauntlet

- **round 1**: slice O1–O3 CHECK plus `cargo test -p draconic-cli --offline` — win. O1 `named-ok` exit 0. O2 `reexport-ok` exit 0. O3 `script-ok` exit 0. Crate `test result: ok.` Promise `toolchain.cli:build-js-library-esm` locked. Review: dropped emit-text reexport assertion; Node import remains the oracle.
