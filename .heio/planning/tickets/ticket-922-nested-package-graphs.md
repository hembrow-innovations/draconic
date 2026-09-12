---
id: "ticket-922-nested-package-graphs"
title: "A library cannot ship its own git dependencies"
kind: ticket
status: open
ticket_type: feature-request
tags: [packages]
blocked_by: []
created_at: "2026-09-13T12:00:00Z"
updated_at: "2026-09-13T12:00:00Z"
---

# A library cannot ship its own git dependencies

## Signal

Lock fill walks only the consumer’s `[dependencies]`. A published library that imports another module path will `get` / `mod tidy` clean for the author, then fail at link for consumers with `NotInLock`. `mod tidy` also drops lock paths that are not in the consumer manifest, so nested pins cannot be stashed. Diamonds are one pin per path with no conflict diagnostic.

## Fit

Unknown until triage. architecture-pkg names direct-deps-only as a v1 trade-off. It still blocks composing libraries. Fits [[location-220-packages]]. Does not rewrite that destination sentence.

## Notes

- `resolve_direct_deps` in `crates/draconic-pkg/src/resolve/resolve_direct.rs`. Test `k04_03_direct_only_ignores_nested_manifest_deps` asserts nested `github.com/transitive/only` never enters the lock.
- `get.rs` and `tidy.rs` parse only the consumer manifest. `ensure_locked_for_entry` only materialises pins already in that lock.
- Linker `resolve_module_import` uses the consumer lock for the whole graph.
- `LockFile.packages` is one entry per module path; `DuplicatePath` rejects two pins.
- Leaf relative internals work (`k06_03`). Re-exporting `from "github.com/other/dep"` does not unless the consumer declared that dep.
- Public Learn/Reference never state this limit.

## Parent

[[location-220-packages]]

## What to build

A library that depends on another git module can be consumed without the consumer hand-flattening those deps, or the limit is diagnosed and documented on the public packages pages.

## Blocked by

none
