---
id: "slice-911-js-library-named-esm"
title: "Opt-in JS named library ESM"
kind: slice
status: met
sprint: "js-library-esm"
blocked_by:
  - slice-909-entry-export-table
tags: [ js, linker, distribution ]
created_at: "2026-09-12T11:05:00Z"
updated_at: "2026-09-12T22:45:00Z"
---
# Opt-in JS named library ESM

## Why

`draconic build --target js` writes a script. Node `import { name } from` that artifact fails. Mixed JS plus Draconic packages need a real named ESM export without copying emit. Default run must stay a Node script.

## Done

`draconic build --target js --library` writes an artifact Node can `import { name } from` for entry named exports, including a two-file re-export under the public name. Without `--library`, `draconic run` and a script-shaped artifact still work. Native `--library` is rejected. Promise `toolchain.cli:build-js-library-esm` is asserted and tested.

## Blocked by

[[slice-909-entry-export-table]]: export names must already survive flatten.

## Non-goals

- **default export and `export *`**
- **Always emitting ESM / `node --input-type=module` as the run host**
- **Multi-file emit without flatten**
- **npm registry**
- **Changing default scratch names**
- **LLVM library artifacts**
- **`cargo test --workspace` as this slice's oracle**

## Oracle checklist

- [x] O1: Node imports a single-file named export
  CHECK: d=$(mktemp -d) && printf '%s\n' 'export const view = "view";' > "$d/lib.drac" && cargo run -p draconic-cli --quiet -- build --target js --library "$d/lib.drac" -o "$d/lib.mjs" && node --input-type=module -e "import { view } from 'file://$d/lib.mjs'; if (view !== 'view') process.exit(1); console.log('named-ok');"
  EXPECT: named-ok
  EVIDENCE: `named-ok`; exit 0
- [x] O2: Node imports a re-export under the public name
  CHECK: d=$(mktemp -d) && printf '%s\n' 'export const view = "view";' > "$d/dep.drac" && printf '%s\n' 'export { view } from "./dep.drac";' > "$d/lib.drac" && cargo run -p draconic-cli --quiet -- build --target js --library "$d/lib.drac" -o "$d/lib.mjs" && node --input-type=module -e "import { view } from 'file://$d/lib.mjs'; if (view !== 'view') process.exit(1); console.log('reexport-ok');"
  EXPECT: reexport-ok
  EVIDENCE: `reexport-ok`; exit 0
- [x] O3: default run of a Module with named exports stays a script
  CHECK: d=$(mktemp -d) && printf '%s\n' 'export const view = "view";' 'console.log("script-ok");' > "$d/p.drac" && cargo run -p draconic-cli --quiet -- run --target js "$d/p.drac"
  EXPECT: script-ok
  EVIDENCE: `script-ok`; exit 0

## Pool

- [[task-912-js-library-named-esm]]

## See also

[[ticket-908-js-library-esm-export]] [[slice-909-entry-export-table]] [[js-library-esm]] [[Toolchain purpose]] [[Toolchain — Contract]] [[0002-shared-ir-dual-backends]] [[0009-go-style-git-packages]] [[location-224-distribution]]
