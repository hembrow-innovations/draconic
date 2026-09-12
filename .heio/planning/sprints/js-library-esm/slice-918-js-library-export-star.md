---
id: "slice-918-js-library-export-star"
title: "JS library export star"
kind: slice
status: frozen
sprint: "js-library-esm"
blocked_by:
  - slice-916-js-library-default-export
tags: [js, linker, distribution]
created_at: "2026-09-12T18:20:00Z"
updated_at: "2026-09-12T18:20:00Z"
---

# JS library export star

## Why

`--library` already exposes named exports and named re-exports. A Module that authors `export *` still does not put those names on the JS artifact, so Node `import { name } from` fails.

## Done

`draconic build --target js --library` of a Module that authors `export * from` writes an artifact Node can `import { name } from` for the re-exported names. `default` is not re-exported. Without `--library`, default build and run stay scripts.

## Blocked by

[[slice-916-js-library-default-export]]: default on the entry table lands first so star work does not fight that skip.

## Non-goals

- **Always emitting ESM / changing `draconic run`**
- **`export * as ns from` as a new form** (named namespace re-export is already a public name)
- **Multi-file emit without flatten**
- **npm registry**
- **LLVM library artifacts**
- **`cargo test --workspace` as this slice's oracle**

## Oracle checklist

- [ ] O1: Node imports a star-re-exported name
  CHECK: d=$(mktemp -d) && printf '%s\n' 'export const view = "view";' > "$d/dep.drac" && printf '%s\n' 'export * from "./dep.drac";' > "$d/lib.drac" && cargo run -p draconic-cli --quiet -- build --target js --library "$d/lib.drac" -o "$d/lib.mjs" && node --input-type=module -e "import { view } from 'file://$d/lib.mjs'; if (view !== 'view') process.exit(1); console.log('star-ok');"
  EXPECT: star-ok
  EVIDENCE: pending
- [ ] O2: star does not re-export default
  CHECK: d=$(mktemp -d) && printf '%s\n' 'export default "def";' 'export const view = "view";' > "$d/dep.drac" && printf '%s\n' 'export * from "./dep.drac";' > "$d/lib.drac" && cargo run -p draconic-cli --quiet -- build --target js --library "$d/lib.drac" -o "$d/lib.mjs" && node --input-type=module -e "import { view } from 'file://$d/lib.mjs'; if (view !== 'view') process.exit(1); const m = await import('file://$d/lib.mjs'); if (m.default !== undefined) process.exit(1); console.log('star-no-default-ok');"
  EXPECT: star-no-default-ok
  EVIDENCE: pending
- [ ] O3: default run of a Module with export star stays a script
  CHECK: d=$(mktemp -d) && printf '%s\n' 'export const view = "view";' > "$d/dep.drac" && printf '%s\n' 'export * from "./dep.drac";' 'console.log("script-ok");' > "$d/p.drac" && cargo run -p draconic-cli --quiet -- run --target js "$d/p.drac"
  EXPECT: script-ok
  EVIDENCE: pending

## Pool

- [[task-919-js-library-export-star]]

## See also

[[ticket-915-js-library-export-star]] [[ticket-908-js-library-esm-export]] [[slice-911-js-library-named-esm]] [[slice-916-js-library-default-export]] [[js-library-esm]] [[Toolchain purpose]] [[Toolchain — Contract]] [[0002-shared-ir-dual-backends]] [[location-224-distribution]]
