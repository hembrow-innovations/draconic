---
id: "ticket-17-roadmap-git-package-manager"
title: "Expand Roadmap for Go-style git package manager"
kind: ticket
status: closed
tags: []
created_at: "2026-07-26T00:00:00Z"
updated_at: "2026-07-26T00:00:00Z"
---

# Expand Roadmap for Go-style git package manager

Archived from `docs/planning/issues/closed/issues-17-roadmap-git-package-manager.md`.

# Expand Roadmap for Go-style git package manager

## Description

Expand [`ROADMAP.md`](../../../ROADMAP.md) so the toolchain can resolve, fetch, and build **dependencies from git repository URLs** in the spirit of Go (`go get` / module paths backed by VCS), not a central npm-style registry as the primary source of truth.

Goal: a Program declares deps by git URL (and version/ref); the CLI clones/fetches into a cache, records a lockfile, and the Frontend/Linker resolve imports against those packages — end-to-end `draconic build` with remote deps.

## Affected

- `ROADMAP.md` (new section e.g. **P — Packages** or Tooling children; Loop source of truth)
- `crates/draconic-cli` (`draconic get` / `draconic mod` / equivalent)
- Frontend + Linker (import graph across package roots)
- Cache layout on disk; lockfile format
- Manifest format (new file vs extend existing)
- CI / offline / reproducibility story
- Related: [[issues-5-ship-real-program]], [[issues-7-new-roadmap-phase]], [[issues-16-roadmap-e2e-http-server]] (server demos will want publishable libs)

## Observed

- ESM import/export exists (E11.*) for **static relative** paths within a Program graph; no external package identity.
- No package manifest, lockfile, module cache, or VCS fetch in the toolchain.
- No Roadmap rows for packaging or dependency management.
- Go model reference: module path + version, proxy optional, **VCS (git) as canonical source**, `go.sum`-style integrity, MVS or equivalent version selection.

## Impact

Without a package story, the ecosystem cannot share libraries; every Program is a monorepo of relative files. HTTP server and product demos cannot pull real deps. Loop has nothing to implement for distribution.

## Proposed Fix

### 1. Human decisions (gate expansion)

1. **Identity model:**
   - **A — URL-as-path (Go-like):** import/`require` path is (or maps 1:1 from) the git hosting path, e.g. `github.com/org/pkg` or full `https://…`.
   - **B — Manifest alias:** short names in manifest map to git URLs; source imports use short names.
   - **C — Hybrid:** URL or path in manifest; imports use package name field.
2. **Version selection:** git tag/semver | commit SHA only | branch (discouraged) | Go-like pseudo-versions.
3. **Lockfile:** required always | optional until first build | checksum algorithm (SHA-256 of tree vs commit OID only).
4. **Manifest filename & schema:** e.g. `draconic.toml` / `Draconic.toml` / `package.drac.json` — fields: module path, deps `{ path/url → version }`, toolchain version pin.
5. **CLI surface:** `draconic get <url>@<ver>` | `draconic mod tidy` | build auto-fetches missing | all of the above.
6. **Registry:** git-only v1 (no central index) | optional later mirror/proxy (like Athens/GOPROXY).
7. **Scope of v1 “done”:** fetch+lock+resolve+build one external git dep (HTTPS, public repo, tagged) on js and/or native.
8. **Roadmap home:** new **P — Packages** section | Tooling **U** children | Phase 2 per [[issues-7-new-roadmap-phase]].

### 2. Seed atomic Roadmap rows (after decisions)

Draft shape (IDs illustrative; each row one Loop; Tests column required):

| Cluster | Example atoms |
|---------|----------------|
| **Manifest** | Parse/write package manifest; schema validation; diagnostics |
| **Lockfile** | Record resolved URL + ref/OID + content hash; stable serialize |
| **Module cache** | On-disk cache layout; clone/fetch git; checkout ref; reuse cache |
| **Version resolve** | Tag/semver → commit; fail closed on ambiguous/missing |
| **CLI get/tidy** | Add dep, update lock, prune unused (as designed) |
| **Import resolve** | Linker/Frontend: package root + subpath → files; no escape outside package |
| **Build integration** | `draconic build` fetches if missing (or errors with fixit); offline mode |
| **Integrity** | Verify lock hashes; refuse tampered cache |
| **E2E fixture** | Temp git repo as dep; consumer Program builds; assert emit/run |
| **Docs example** | Minimal publishable lib + consumer in `examples/` |

Out of v1 (file later children): private git auth matrix, monorepo multi-module, replace/fork directives, central registry, yank, license scanning, binary plugins.

### 3. Execution checklist (agent after human brief)

1. Add section + legend to `ROADMAP.md` without deleting B/E/T/N/U history.
2. Insert atomic `todo` rows with **Targets** (`compiler` / `both` as appropriate) and **Tests** paths.
3. ADR for identity + lockfile + cache layout (link from Comments).
4. Cross-link [[issues-16-roadmap-e2e-http-server]] and product issues.
5. Roadmap expansion only in the first change set unless a pilot row is explicitly in scope.

## Human Brief

### Goal

Decide package identity, versioning, lockfile, manifest, CLI, and git-only v1 bar — then expand `ROADMAP.md` with Loop-sized rows for a Go-style git-URL package manager.

### Why human

Ecosystem shape (URL-as-path vs npm-like names), lockfile strictness, and auto-fetch-on-build are product/security judgment. Wrong defaults are costly to reverse.

### Decisions needed

1. Identity: **A** URL/path | **B** alias | **C** hybrid (recommend **A** or **C** with Go-like paths).
2. Versions: tags/semver + commit pin in lockfile (recommend).
3. Manifest + lockfile names/schema.
4. CLI commands and whether build auto-fetches.
5. v1 success demo (public HTTPS git dep).
6. Roadmap section home.

### After decisions

- Agent expands `ROADMAP.md` + optional ADR.
- Loop implements test-first (fixture git repos under `tests/`).
- Example lib+consumer when resolve+build cluster is green.

### Context (verified 2026-07-26)

- E11 modules: static relative only; no package graph.
- No packaging ADRs; toolchain is compile+runtime only.
- Go reference model: VCS-backed modules, sumdb-like integrity optional later.

### Out of scope

- npm/cargo registry compatibility as v1 requirement.
- Implementing full package manager in one PR.
- Replacing ESM with a different module syntax (resolve *to* ESM files inside packages).

## Comments

Related: [[issues-5-ship-real-program]], [[issues-7-new-roadmap-phase]], [[issues-16-roadmap-e2e-http-server]]

Model reference: Go modules (`go get`, module path, `go.sum`, git tags) — adapt to Draconic ESM + dual backends, not copy `GOPATH` era.

> **2026-07-26 closed:** Human gates locked and executed. Decisions: hybrid module path + optional URL map, semver tags, `draconic.toml`/`draconic.lock`, `get`/`mod tidy`, auto-fetch + `--offline`, git-only v1, section **K**. Seeded Roadmap **K01–K11**; ADR-0009. Package manager implementation is Loop work on those rows—not this issue.
