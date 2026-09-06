---
id: "architecture-linker"
title: "Linker"
kind: architecture
description: "Load an ESM import graph, mangle bindings, and flatten to one Program, with package-path imports via the lock."
domain: draconic
area: tooling
tags: []
created_at: "2026-09-06"
updated_at: "2026-09-06"
---

# Linker

## Overview

`draconic-linker` loads an entry path’s ESM import graph, mangles per-module top-level bindings, rewrites import locals to exporter names, and concatenates dependency bodies into one Program. [[CONTEXT]] defines Linker this way. Frontend chooses parse versus link; the Parser product does not own this step. See [[architecture-frontend]] and [[architecture-pipeline]].

## Context

Draconic Programs use ESM `import` / `export`. Both backends consume one IR Module, so the graph must become a single Program before check and lower. A bundler-shaped tool outside the compiler would duplicate resolution and miss live-binding and package-boundary rules. Package identity stays in [[architecture-pkg]]; this crate only resolves specifiers onto files and flattens.

## Design

Public entry: `link_entry(path)` discovers `draconic.lock` by walking ancestors and uses the default module cache under that workspace. `link_entry_with_packages` takes an explicit lock + cache. No lock means only relative specifiers succeed.

Specifier policy (`resolve_specifier`):

- **relative** `./` or `../`: join, canonicalize, then if the importer sits in a package checkout (`.draconic-checkout-oid`), `ensure_within_package` rejects escape (K06.02).
- **module path** (`github.com/org/pkg` and subpaths): `resolve_module_import` against the active package context (K06.01). Missing context is an error. Integrity is checked at resolve time.
- anything else: error (“only relative module specifiers are supported”).

Graph load: Module goal parse (`.json` files become JSON modules with a default export). Static import/export, `export *`, `export * as`, `export { … } from`, `import defer`, and string-literal `import()` / `import.defer()` pull dependencies. Cycles are allowed (live bindings via shared cells). Named, default, and namespace imports are recorded.

Link: non-entry modules mangle top-level names as `__m{id}_{name}`; the entry keeps source local names. Imports rewrite to the defining module’s mangled binding. Eager evaluation is the entry plus `eval_deps` (deferred-only edges stay lazy unless TLA forces them). Shared namespace objects (`__ns{id}`) and deferred namespaces are synthesised. Output is one `Program` whose body is helper stmts plus eager module bodies in InnerModuleEvaluation order.

Relative imports and module-path imports coexist in one graph (K06.03): consumer files may import `github.com/…` while package internals keep `./` relatives inside the checkout.

## Trade-offs

Flattening before check means one Program and simple backends, at the cost of a large linker (synthetic spans, deferred namespaces, async TLA interleaving). It is a static link, not a runtime loader: only string-literal dynamic import targets in the graph are rewritten. Package fetch is not the linker’s job; `build` must `ensure` first.

## Consequences

[[architecture-cli]] `check` / `build` / `run` go through [[architecture-frontend]] `compile_path` / `check_path`, which call `link_entry` when the entry has module syntax. [[architecture-lsp]] analyzes a single Script buffer and does not link. Editors must not treat this crate as a bundler.
