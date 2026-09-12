---
id: "task-919-js-library-export-star"
title: "JS library export star"
kind: task
status: ready
mode: afk
blocked_by:
  - task-917-js-library-default-export
sprint: "js-library-esm"
slice: "slice-918-js-library-export-star"
tags: [js, linker, distribution]
created_at: "2026-09-12T18:20:00Z"
updated_at: "2026-09-12T18:20:00Z"
---

# JS library export star

## Blocked by

[[task-917-js-library-default-export]]: default on the entry table lands first so star expansion does not fight that skip.

## Done

`draconic build --target js --library` writes an artifact Node can `import { name } from` for `export * from` names. Slice O1–O3 hold. Promise `toolchain.cli:build-js-library-export-star` is asserted and tested.

## Context

Current vs desired, interfaces, and out of scope live in the agent brief. Named `export { view } from` already works. Star names never enter the IR table.

Always `draconic build … -o` into a temp path. Do not emit `{stem}.out.js` beside fixtures or examples.

## Verify

Slice O1–O3 CHECK print their EXPECT tokens and exit 0. `cargo test -p draconic-cli --offline` prints `test result: ok.`

scope: Linker entry export table for `export *` star names, JS library wrapper if needed, toolchain contract and test map for `toolchain.cli:build-js-library-export-star`, CLI tests. Not LLVM emit. Not `draconic run` host change.

## Links

[[slice-918-js-library-export-star]] [[ticket-915-js-library-export-star]] [[task-917-js-library-default-export]] [[js-library-esm]]

## Agent Brief

**Category:** enhancement
**Summary:** Opt-in `draconic build --target js --library` so Node can `import { name } from` names brought in by `export *`.

**Drain:** `/afk-task`. Blocked until [[task-917-js-library-default-export]] is `completed`. Claim only when unblocked.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Skills:** load **tdd**, **draconic-language**, **rust-development**. Compiler sitting, not a Roadmap atom.

**Intent (required when product behaviour changes):**
- This **is** product behaviour (library star names on the JS artifact). **No product behaviour change** is false.
- Promise ids: assert `toolchain.cli:build-js-library-export-star`. Keep `toolchain.cli:build-js-library-esm`, `toolchain.cli:build-js-library-default`, `toolchain.cli:build-targets`, `toolchain.cli:build-scratch-name`, and `toolchain.cli:run-execute`. Do not invent npm or package-resolve promises.
- Purpose: [[Toolchain purpose]]. Dual backends [[0002-shared-ir-dual-backends]]. Packages stay git modules [[0009-go-style-git-packages]]. Distribution location [[location-224-distribution]] is not rewritten.
- Contract-first: edit [[Toolchain — Contract]] and [[Toolchain tests]] for the new promise, then lock with Node `import { name }`, then implement. Do not treat `js.check` append as the oracle.

**Current behavior:**
`draconic build --target js --library` on an entry `export * from "./dep.drac"` whose dependency is `export const view = "view"` writes `const __m0_view = "view"` with no `export`. Node `import { view } from` fails. Linker `entry_named_exports` walks direct exports and named re-exports only; it does not include `export *` star names. Named `export { view } from` already works from [[task-912-js-library-named-esm]].

**Desired behavior:**
`draconic build --target js --library <entry> -o <temp>`:
- Entry `export * from "./dep.drac"` with dep `export const view = "view"` → Node `import { view }` yields `"view"`. Public name is `view`, not `__m0_view`.
- Star does not re-export `default`. A dep that authors `export default` plus named exports still exposes the named names only.
- Ambiguous star collisions stay omitted (same as linker GetModuleNamespace), not a new error class.
- Without `--library`, `draconic run --target js` of a Module that authors `export *` still runs as a script and prints.
- Named `--library` emit and default `--library` emit still hold.

Expand star names onto the existing IR named-export table. The current `export { local as public }` wrapper should be enough. Do not restore `Stmt::Export` through check. Do not add a second IR. Do not treat `export * as ns from` as this sitting; that form is already a public name `ns`.

**Key interfaces:**
- Linker `entry_named_exports` and `star_reexports` / `collect_resolved_exports`. Skip `default` when it would arrive only through star.
- IR `Module.named_exports`. LLVM still ignores it.
- JS `emit_js_library`. Do not print `export` on default `emit_js`.
- Node verification must `import` the file (`--input-type=module` or `.mjs`).

**Acceptance criteria:**
- [ ] Slice O1 CHECK prints `star-ok` and exits 0
- [ ] Slice O2 CHECK prints `star-no-default-ok` and exits 0
- [ ] Slice O3 CHECK prints `script-ok` and exits 0
- [ ] `toolchain.cli:build-js-library-export-star` is on [[Toolchain — Contract]] with a `test:` pointer, and [[Toolchain tests]] names that test
- [ ] `cargo test -p draconic-cli --offline` prints `test result: ok.`
- [ ] Promise ids listed above still hold (or were deliberately edited)

**Out of scope:**
- Always emitting ESM; changing `draconic run` to `node --input-type=module`
- Keeping `export` statements through check and lower
- Multi-file ESM without flatten
- npm as v1 packages
- LLVM library emit
- `cargo test --workspace` as the unit oracle
