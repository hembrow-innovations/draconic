---
id: "purpose"
title: "Packages purpose"
kind: purpose
description: "Product brief: job, scope, and fences for Go-style git-backed modules."
status: active
domain: draconic
area: packages
tags: [purpose]
created_at: "2026-09-06"
updated_at: "2026-09-06"
---

# Packages purpose

## Job

Let a Program depend on git-backed modules identified by a Go-like module path, with a lock that pins commit and tree hash, without a central registry.

## In scope

- **Hybrid identity**: imports use a module path such as `github.com/org/pkg`; `draconic.toml` may map path → git URL when default `https://{module_path}.git` is wrong.
- **Manifest and lock**: `draconic.toml` (module path, deps, optional URL map); `draconic.lock` pins version, git URL, commit OID, and SHA-256 of the package tree.
- **Cache**: clone/fetch into a module cache; checkout by OID; cache hit skips network.
- **Integrity**: recompute tree hash; refuse mismatched OID or tampered cache.
- **CLI**: `draconic get`, `draconic mod tidy`; `draconic build` auto-fetches missing locked deps unless `--offline`.
- **v1 bar**: K01–K08 plus K09.02 e2e. K11 children are opt-in later knobs, never the silent default fetch path.

## Out of scope

- **Central npm-like registry**: no crates.io clone and no registry as v1 primary ([[0009-go-style-git-packages]]).
- **Lockfile-optional floating builds**: when a lock is present, build uses lock pins.
- **Replacing ESM**: resolve still lands on ESM files inside a checkout; Linker loads those files.
- **Language semantics** and **host I/O catalogue**: other spec folders.

## Surfaces

- **Files**: `draconic.toml`, `draconic.lock`, module cache under `.draconic/mod-cache` or `DRACONIC_MOD_CACHE`.
- **CLI**: `draconic get`, `draconic mod tidy`, `draconic build --offline`.
- **Imports**: `from "github.com/org/pkg"` (and subpath) via Linker plus cache.

## Authority

- Behaviour: [[Packages — Contract]]
- Tests: [[Packages tests]]
- Glossary: [[CONTEXT]]
- Decisions: [[0009-go-style-git-packages]]
- Shape: [[architecture-pkg]], [[architecture-linker]], [[architecture-cli]]

## Open product questions

- (none)
