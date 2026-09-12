---
id: "ticket-908-js-library-esm-export"
title: "JS build emit has no ESM export for library modules"
kind: ticket
status: promoted
ticket_type: feature-request
tags: [js, linker, distribution]
blocked_by: []
sprint: "js-library-esm"
created_at: "2026-09-12T10:50:00Z"
updated_at: "2026-09-12T11:05:00Z"
---

# JS build emit has no ESM export for library modules

## Signal

Inbound GitHub issue 1 (`hembrow-innovations/draconic`, 2026-09-12). `draconic build --target js` on a Module that uses `export const`, `export function`, or `export { name }` writes a script with the `export` keyword gone. Linked graphs flatten and rename (`__m0_view`) with no remaining ESM export.

This blocks a mixed JS plus Draconic package (dragonflame-ui task-285): at least one public named export must be authored in Draconic, and JavaScript callers must `import` the sibling JS-backend emit. Node `import { view } from "dragonflame-ui"` needs a real ESM named export.

Do not work around this in the consumer repo. That repo must not copy or hand-edit JS emit.

## Fit

Promoted to [[slice-909-entry-export-table]] then [[slice-911-js-library-named-esm]] in [[js-library-esm]]. Executable AFK: [[task-910-entry-export-table]], then [[task-912-js-library-named-esm]]. Does not rewrite [[location-220-packages]] or [[location-224-distribution]]. Default export and `export *` stay later children, not this sprint. Not AFK from this ticket.

## Notes

- Reproduced on this checkout. `export const view = "view"` emits `const view = "view"`. A two-file re-export emits `const __m0_view = "view"`. `tests/conformance/fixtures/es/modules/named_lib.drac` emits `let value` / `function inc` / `const label` with no `export`. `examples/pkg-lib/index.drac` emits `const VERSION` / `function greet` with no `export`.
- Node `import { view } from` that artifact fails: named export `view` not found; Node treats the file as CommonJS. `node artifact` as a script exits 0. That matches `draconic run`, which spawns `node` on the artifact with no `--input-type=module`.
- This is not a JS pretty-printer dropping a keyword. The pipeline peels `import` / `export` on purpose, then both backends print a script-shaped IR.
- Frontend `load_program` calls `link_entry` whenever the entry AST has `ImportDeclaration` or any `Export*` statement, even a single-file library with only `export`.
- Linker load (`crates/draconic-linker/src/load.rs`, `ModuleData`) records `export_name → local` then pushes the inner `let` / `const` / `function` / `class` only. Import statements never enter `body`. Link (`link.rs`) mangles non-entry top-level names to `__m{id}_{name}`; the entry keeps source names for `js.check`. `link_entry` returns one `Program`. The export table is dropped at that boundary.
- Checker rejects leftover `import` / `export` with `import/export must be linked before bind/check`. Lower panics `import/export must be linked before lower`. IR `Module` has locals, body, spans, shapes, `has_extern_ffi`. No export list. JS backend has no `export` printer. LLVM `es_modules` is the same flatten, different backend.
- `compile_source_module` does not link and therefore cannot emit a library either: leftover `export` fails check.
- Conformance never locks emit shape. `named_lib.drac` has no `.meta`; `named_export_import` links the consumer, appends `js.check`, and runs Node as a script. E11 is `done` on program results. K10 `pkg-lib` is an exportable *source* module for Draconic consumers, not a Node ESM artifact.
- Locked design this must not fight: [[0002-shared-ir-dual-backends]] (one IR after Frontend), [[0009-go-style-git-packages]] (git modules, resolve *to* ESM *source* files, not npm as v1), CONTEXT Linker (flatten to one Program; not a bundler), `pipe-linker-not-parser`, `ir-after-link`.
- CLI today: `--target js|native`, `-o`, `--watch`, `--offline`, native `--strip` / `--lto` / `--link`. No `--library`, `--emit esm`, `--no-flatten`, or `--format`.
- Putting `export` on the default JS artifact would SyntaxError `draconic run` and the conformance `node -e` harness unless those hosts become modules too.

## Parent

[[slice-909-entry-export-table]] [[slice-911-js-library-named-esm]]

Inbound GitHub issue 1. Adjacent closed work: [[ticket-11-linker-own-module]] (flatten is the Linker product), [[ticket-17-roadmap-git-package-manager]] (packages resolve onto ESM source files).

## What to build

End-to-end: a Draconic library entry that authors a public named export can be built to JavaScript that Node can `import { name } from`. A two-file graph that re-exports must expose the public name, not `__m0_*`. Default `draconic run` and the current conformance script host must keep working. dragonflame-ui must not copy emit.

## Possible remedies

Options for triage. Not a locked design.

- **Library ESM wrapper on flattened emit (recommended first cut).** Keep linker flatten (cycles, live bindings, git packages). Carry the *entry* export table through IR as metadata, not as `Stmt::Export`. In an opt-in JS library mode, append `export { local as publicName }` and `export default` when present. LLVM ignores the metadata. `draconic run` stays a Node script. Tests: Node `import { view } from` the artifact; re-export graph exposes `view`; `named_lib` exposes `value`, `inc`, `label`. Fits one IR. Does not restore export statements before check.
- **Always emit ESM and run with `node --input-type=module`.** Same wrapper, no opt-in. Bigger blast: conformance `node -e` plus `js.check`, scratch `{stem}.out.js`, and every current JS program become modules. Do not take this as the first cut.
- **Keep `export` statements through check and lower.** Would add IR export stmts and undo `ir-after-link`. Fights the flatten-before-check architecture. Do not take this.
- **Multi-file ESM emit without flatten.** Each `.drac` becomes a JS file with real `import` / `export`. Second JS pipeline. Native still needs flatten. Cycles, live bindings, JSON modules, and package resolve would be reimplemented. Do not take this as the first cut.
- **Consumer workaround.** Hand-wrap or copy JS emit in dragonflame-ui. Forbidden by the signal.

## Recommendations

- Treat this as missing *library* emit, not a printer bug. E11 remaining green is not a false green for *run*; it never promised Node-importable artifacts.
- First slice: named exports on a flattened bundle, opt-in CLI, tests that `import` the file. Default export and `export *` can be later children.
- Do not change `draconic run` in that slice. Do not skip Frontend. Do not add a second IR. Do not introduce npm as the package story.
- Oracle should be Node `import { … }` against sibling emit, not another `js.check` append. Building `named_lib.drac` as an entry should be a first-class test; today it is only a dependency.

## Blocked by

- none

## Comments

> *This was generated by AI during triage.*

## Triage Notes

**What we've established so far:**

- Missing library emit, not a printer bug. JS backend has no `export` printer. Linker flatten drops the entry export table. E11 never promised Node-importable artifacts. No docs rejection; ADR-0009 rejects npm as v1, not this.
- First cut: named exports on flattened JS emit, opt-in `--library`. Default `draconic run` stays a script. Default export and `export *` stay out of this sprint.
- Remainder after slice-911: [[ticket-913-default-js-build-no-esm]], [[ticket-914-js-library-default-export]], [[ticket-915-js-library-export-star]].
- Promoted. Drain [[task-910-entry-export-table]] first.
