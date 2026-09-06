---
title: Git-backed packages
impact: MEDIUM
impactDescription: npm as v1 primary forks identity and lock
tags: [pkg, git, modules]
---

## Git-backed packages

Packages are git-backed. Imports use a Go-like module path. Manifest is `draconic.toml`. Lockfile is `draconic.lock` (commit OID + SHA-256 tree hash).

**Incorrect:** `package.json` / npm install as the v1 package story, or lockfile-optional floating builds.

**Correct:**

```bash
draconic get
draconic mod tidy
draconic build --target js --offline app.drac
```

**Notes.** ADR-0009. `draconic.toml` may map path → git URL. Versions are semver git tags. Resolve *to* ESM files inside packages; do not replace ESM syntax. See `pipe-linker-not-parser`.
