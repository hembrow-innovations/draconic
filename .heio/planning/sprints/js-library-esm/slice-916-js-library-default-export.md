---
id: "slice-916-js-library-default-export"
title: "JS library default export"
kind: slice
status: frozen
sprint: "js-library-esm"
blocked_by: []
tags: [js, linker, distribution]
created_at: "2026-09-12T18:20:00Z"
updated_at: "2026-09-12T18:20:00Z"
---

# JS library default export

## Why

`--library` already exposes named exports. A Module that authors `export default` still has no default on the JS artifact, so Node `import d from` fails.

## Done

`draconic build --target js --library` of a Module that authors `export default` writes an artifact Node can `import d from`. Named exports on the same entry still import. Without `--library`, default build and run stay scripts.

## Blocked by

None.

## Non-goals

- **`export *`**: [[slice-918-js-library-export-star]]
- **Always emitting ESM / changing `draconic run`**
- **Multi-file emit without flatten**
- **npm registry**
- **LLVM library artifacts**
- **`cargo test --workspace` as this slice's oracle**

## Oracle checklist

- [ ] O1: Node imports a default export
  CHECK: d=$(mktemp -d) && printf '%s\n' 'export default "def";' > "$d/lib.drac" && cargo run -p draconic-cli --quiet -- build --target js --library "$d/lib.drac" -o "$d/lib.mjs" && node --input-type=module -e "import v from 'file://$d/lib.mjs'; if (v !== 'def') process.exit(1); console.log('default-ok');"
  EXPECT: default-ok
  EVIDENCE: pending
- [ ] O2: Node imports default plus a named export
  CHECK: d=$(mktemp -d) && printf '%s\n' 'export default "def";' 'export const named = "n";' > "$d/lib.drac" && cargo run -p draconic-cli --quiet -- build --target js --library "$d/lib.drac" -o "$d/lib.mjs" && node --input-type=module -e "import v, { named } from 'file://$d/lib.mjs'; if (v !== 'def' || named !== 'n') process.exit(1); console.log('default-named-ok');"
  EXPECT: default-named-ok
  EVIDENCE: pending
- [ ] O3: default run of a Module with export default stays a script
  CHECK: d=$(mktemp -d) && printf '%s\n' 'export default "def";' 'console.log("script-ok");' > "$d/p.drac" && cargo run -p draconic-cli --quiet -- run --target js "$d/p.drac"
  EXPECT: script-ok
  EVIDENCE: pending

## Pool

- [[task-917-js-library-default-export]]

## See also

[[ticket-914-js-library-default-export]] [[ticket-908-js-library-esm-export]] [[slice-911-js-library-named-esm]] [[js-library-esm]] [[Toolchain purpose]] [[Toolchain — Contract]] [[0002-shared-ir-dual-backends]] [[location-224-distribution]]
