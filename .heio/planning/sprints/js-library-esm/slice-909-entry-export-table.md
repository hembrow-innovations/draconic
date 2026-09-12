---
id: "slice-909-entry-export-table"
title: "Entry named exports survive flatten"
kind: slice
status: met
sprint: "js-library-esm"
blocked_by: []
tags: [js, linker, distribution]
created_at: "2026-09-12T11:05:00Z"
updated_at: "2026-09-12T21:20:00Z"
---

# Entry named exports survive flatten

## Why

Linker flatten drops the entry export table before check and lower. JS library emit cannot name public exports later if that table is gone. This cut keeps entry named-export names on the shared IR as metadata. LLVM ignores them. Default JS emit still has no `export`.

## Done

After Frontend `compile_path` of a Module entry that authors named exports, the IR `Module` still lists each public name and the local it binds after flatten. A two-file re-export lists the public name, not a dropped table. No CLI flag. No `export` in default JS print.

## Blocked by

None.

## Non-goals

- **Opt-in JS `export { … }` printer and CLI**: [[slice-911-js-library-named-esm]]
- **default export and `export *`**
- **LLVM using or printing the table**
- **Restoring `Stmt::Export` through check and lower**
- **Changing `draconic run`**
- **`cargo test --workspace` as this slice's oracle**

## Oracle checklist

- [x] O1: single-file named export names survive lower
  CHECK: cargo test -p draconic-frontend --offline entry_named_exports_on_ir -- --nocapture
  EXPECT: view-export-ok
  EVIDENCE: `view-export-ok`; test result: ok. 1 passed; exit 0
- [x] O2: re-export graph keeps the public name on IR
  CHECK: cargo test -p draconic-frontend --offline reexport_entry_export_names_on_ir -- --nocapture
  EXPECT: view-reexport-ok
  EVIDENCE: `view-reexport-ok`; test result: ok. 1 passed; exit 0

## Pool

- [[task-910-entry-export-table]]

## See also

[[ticket-908-js-library-esm-export]] [[js-library-esm]] [[0002-shared-ir-dual-backends]] [[architecture-ir]] [[architecture-linker]] [[architecture-frontend]] [[Toolchain purpose]]
