---
id: "task-910-entry-export-table"
title: "Keep entry named exports on IR"
kind: task
status: completed
mode: afk
blocked_by: []
sprint: "js-library-esm"
slice: "slice-909-entry-export-table"
tags: [js, linker, distribution]
created_at: "2026-09-12T11:05:00Z"
updated_at: "2026-09-12T11:14:46Z"
---

# Keep entry named exports on IR

## Blocked by

None.

## Done

Frontend `compile_path` of a named-export entry leaves those public names on the IR `Module`. Slice O1 and O2 hold.

## Context

Current: Linker records `export_name → local` while loading, then `link_entry` returns one `Program` with import/export statements peeled. IR `Module` has locals, body, spans, shapes, `has_extern_ffi`. No export list. Both backends print a script.

Desired: After flatten, check, and lower, the IR `Module` still carries the *entry* named-export table as metadata (public name plus the local that holds the value after mangle). LLVM ignores it. Default JS emit still has no `export` keyword.

Out of scope: CLI `--library`, JS `export { … }` printer, default export, `export *`, restoring `Stmt::Export` through check, changing `draconic run`, npm.

Always `draconic build … -o` into a temp path if you build. Do not emit `{stem}.out.js` beside fixtures.

## Verify

Slice O1 and O2 CHECK print their EXPECT tokens and exit 0. `cargo test -p draconic-frontend --offline` prints `test result: ok.`

scope: shared IR `Module` metadata, Linker entry export table through Frontend `compile_path` / lower, Frontend crate tests named in the slice oracles. Not CLI. Not JS printer. Not LLVM emit.

## Links

[[slice-909-entry-export-table]] [[ticket-908-js-library-esm-export]] [[js-library-esm]]

## Agent Brief

**Category:** enhancement
**Summary:** Carry the entry module's named-export table through flatten onto the shared IR as metadata, with no JS `export` printer yet.

**Drain:** `/afk-task`. Unblocked. Claim `status: ready` then `mode: afk`.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Skills:** load **tdd**, **draconic-language**, **rust-development**. Compiler sitting, not a Roadmap atom.

**Intent (required when product behaviour changes):**
- **No product behaviour change.** Metadata only. CLI, JS emit, and `draconic run` stay as they are.
- Promise ids: none new. Do not invent a CLI promise on this task.
- Purpose: [[Toolchain purpose]] facade (`toolchain.frontend:facade`). Dual backends [[0002-shared-ir-dual-backends]]. Linker flatten [[architecture-linker]].
- Contract-first: add the Frontend tests named in slice O1 and O2 first, then carry the table.

**Current behavior:**
A Module entry with `export const`, `export function`, or `export { name }` is linked. The linker keeps an export map while loading, then returns one `Program` without export statements. `lower` produces an IR `Module` with no export list. Check rejects leftover `import` / `export`.

**Desired behavior:**
`compile_path` on a single-file `export const view = "view"` yields an IR `Module` whose export metadata includes public name `view` bound to local `view`. `compile_path` on an entry `export { view } from "./dep.drac"` whose dependency is `export const view = "view"` yields metadata whose public name is `view` bound to the flattened local (mangled dependency name is fine; dropping the public name is not). Direct `export { local as publicName }` uses `publicName` as the public key. LLVM still ignores the field. Default `emit_js` still prints no `export` keyword. File-size budget still holds.

**Key interfaces:**
- IR `Module`. Add metadata for entry named exports (public name → local name after flatten). Not `Stmt::Export`. Not a second IR.
- Linker already has `export_name → local` plus named re-exports on load. `link_entry` must not drop the *entry* named exports before Frontend lower can copy them.
- Frontend `compile_path` / `compile_path_for_target` remain the compile entry. Callers still do not wire parser, checker, and IR by hand.
- Checker still rejects leftover `import` / `export` on the flattened `Program`.

**Acceptance criteria:**
- [x] Slice O1 CHECK prints `view-export-ok` and exits 0
- [x] Slice O2 CHECK prints `view-reexport-ok` and exits 0
- [x] `cargo test -p draconic-frontend --offline` prints `test result: ok.`
- [x] Default JS emit of a named-export entry still has no `export` keyword
- [x] No CLI flag added

**Out of scope:**
- `draconic build --library` and the JS `export { … }` printer ([[task-912-js-library-named-esm]])
- default export and `export *`
- Always-ESM run host
- Multi-file emit without flatten
- npm
- Restoring export statements through check and lower
- LLVM printing or consuming the table
- `cargo test --workspace` as the unit oracle
