---
id: "task-917-js-library-default-export"
title: "JS library default export"
kind: task
status: ready
mode: afk
blocked_by: []
sprint: "js-library-esm"
slice: "slice-916-js-library-default-export"
tags: [js, linker, distribution]
created_at: "2026-09-12T18:20:00Z"
updated_at: "2026-09-12T18:20:00Z"
---

# JS library default export

## Blocked by

None.

## Done

`draconic build --target js --library` writes an artifact Node can `import d from` for an authored default export. Slice O1–O3 hold. Promise `toolchain.cli:build-js-library-default` is asserted and tested.

## Context

Current vs desired, interfaces, and out of scope live in the agent brief. Named `--library` emit is already in. Default is skipped in the entry export table.

Always `draconic build … -o` into a temp path. Do not emit `{stem}.out.js` beside fixtures or examples.

## Verify

Slice O1–O3 CHECK print their EXPECT tokens and exit 0. `cargo test -p draconic-cli --offline` prints `test result: ok.`

scope: Linker entry export table for `default`, JS library wrapper, toolchain contract and test map for `toolchain.cli:build-js-library-default`, CLI tests. Not LLVM emit. Not `draconic run` host change. Not `export *`.

## Links

[[slice-916-js-library-default-export]] [[ticket-914-js-library-default-export]] [[task-912-js-library-named-esm]] [[js-library-esm]]

## Agent Brief

**Category:** enhancement
**Summary:** Opt-in `draconic build --target js --library` so Node can `import d from` a flattened default export.

**Drain:** `/afk-task`. Unblocked. Claim `status: ready` then `mode: afk`.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Skills:** load **tdd**, **draconic-language**, **rust-development**. Compiler sitting, not a Roadmap atom.

**Intent (required when product behaviour changes):**
- This **is** product behaviour (library default on the JS artifact). **No product behaviour change** is false.
- Promise ids: assert `toolchain.cli:build-js-library-default`. Keep `toolchain.cli:build-js-library-esm`, `toolchain.cli:build-targets`, `toolchain.cli:build-scratch-name`, and `toolchain.cli:run-execute`. Do not invent npm or package-resolve promises.
- Purpose: [[Toolchain purpose]]. Dual backends [[0002-shared-ir-dual-backends]]. Packages stay git modules [[0009-go-style-git-packages]]. Distribution location [[location-224-distribution]] is not rewritten.
- Contract-first: edit [[Toolchain — Contract]] and [[Toolchain tests]] for the new promise, then lock with Node `import d from`, then implement. Do not treat `js.check` append as the oracle.

**Current behavior:**
`draconic build --target js --library` on `export default "def"; export const named = "n"` writes `let __default = "def";` and `export { named }`. Node `import d from` fails: no export named `default`. Linker `entry_named_exports` skips public name `default`. Flatten already binds the default local (`__default` for anonymous default). Default `draconic run` stays a script.

**Desired behavior:**
`draconic build --target js --library <entry> -o <temp>`:
- Single-file `export default "def"` → Node `import v from` that file yields `"def"`.
- `export default "def"` plus `export const named = "n"` → Node `import v, { named }` yields both.
- Anonymous `export default function` / `export default class` use the existing flattened local (today `__default`).
- Without `--library`, `draconic run --target js` of a Module that authors `export default` still runs as a script and prints. Default scratch names unchanged.
- Named `--library` emit from [[task-912-js-library-named-esm]] still holds.

Prefer carrying `default` on the existing IR named-export table (`public_name` `default`) so the current `export { local as public }` wrapper can emit `export { __default as default }`. Do not restore `Stmt::Export` through check. Do not add a second IR.

**Key interfaces:**
- Linker `entry_named_exports` (skips `default` today). Direct default and `export { x as default }` must survive flatten as metadata.
- IR `Module.named_exports`. LLVM still ignores it.
- JS `emit_js_library`. May keep the named wrapper; do not print `export` on default `emit_js`.
- Node verification must `import` the file (`--input-type=module` or `.mjs`).

**Acceptance criteria:**
- [ ] Slice O1 CHECK prints `default-ok` and exits 0
- [ ] Slice O2 CHECK prints `default-named-ok` and exits 0
- [ ] Slice O3 CHECK prints `script-ok` and exits 0
- [ ] `toolchain.cli:build-js-library-default` is on [[Toolchain — Contract]] with a `test:` pointer, and [[Toolchain tests]] names that test
- [ ] `cargo test -p draconic-cli --offline` prints `test result: ok.`
- [ ] Promise ids listed above still hold (or were deliberately edited)

**Out of scope:**
- `export *` ([[task-919-js-library-export-star]])
- Always emitting ESM; changing `draconic run` to `node --input-type=module`
- Keeping `export` statements through check and lower
- Multi-file ESM without flatten
- npm as v1 packages
- LLVM library emit
- `cargo test --workspace` as the unit oracle
