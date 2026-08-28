---
title: packages
section: learn
status: not-yet
---

# packages

Packages are git-backed. v1 does not require a central registry.

Identity is hybrid: imports use a Go-like module path such as `github.com/org/pkg`. A `draconic.toml` manifest may map that path to a git URL when default URL derivation is wrong.

Versions are semver git tags. The lockfile `draconic.lock` pins commit OID and a content hash of the package tree. `draconic get` and `draconic mod tidy` are the CLI; `draconic build` fetches missing locked deps unless `--offline`.

Resolve lands on ESM files inside the package. That is the join back to [modules](modules.html).
