---
id: "ticket-923-package-no-export-map"
title: "Package public API is every file in the checkout"
kind: ticket
status: open
ticket_type: feature-request
tags: [packages, linker]
blocked_by: []
created_at: "2026-09-13T12:00:00Z"
updated_at: "2026-09-13T12:00:00Z"
---

# Package public API is every file in the checkout

## Signal

There is no exports map, `files` field, or internal hide. Module-path imports plus subpaths resolve onto files in the checkout (`index.drac` / `mod.drac` / `main.drac`, or `util` → `util.drac`). Any `.drac` in the hashed tree is importable.

## Fit

Unknown until triage. Fits [[location-220-packages]]. Not an npm `package.json` exports wishlist. Distinct from [[ticket-915-js-library-export-star]] (JS artifact star names).

## Notes

- Manifest schema in `crates/draconic-pkg` has no `exports`.
- `import_resolve.rs` maps specifiers onto files. `content_hash_tree` hashes every regular file except `.git` and the checkout marker.
- `examples/pkg-lib` is a single `index.drac`. Multi-file subpath exists only in unit tests, not an authoring example.
- K11.03 monorepo subdir is later and is not an exports map.

## Parent

[[location-220-packages]]

## What to build

A library author can declare a public surface so consumers cannot import internals by subpath, or the “all files are public” rule is stated on Learn/Reference.

## Blocked by

none
