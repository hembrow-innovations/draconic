# Go-style git-backed packages

Dependency management lands under Roadmap **K**. Packages are **git-backed** (no central registry required in v1). Identity is **hybrid**: imports use a Go-like **module path** (e.g. `github.com/org/pkg`); `draconic.toml` may map path → git URL when default URL derivation is wrong.

**Versions:** semver git tags. **Lockfile** `draconic.lock` pins commit OID + content hash (SHA-256 of package tree). **Manifest** `draconic.toml`. **CLI:** `draconic get`, `draconic mod tidy`; `draconic build` auto-fetches missing locked deps unless `--offline`.

Rejected as v1 primary: npm-compatible registry, crates.io clone, lockfile-optional floating builds, replacing ESM with a different module syntax (resolve *to* ESM files inside packages).
