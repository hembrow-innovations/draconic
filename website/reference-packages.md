---
title: packages
section: reference
status: not-yet
---

# packages

Packages are git-backed. v1 does not require a central registry.

- Imports use a Go-like module path such as github.com/org/pkg
- draconic.toml may map that path to a git URL
- Versions are semver git tags
- draconic.lock pins commit OID and a content hash of the package tree
- draconic get and draconic mod tidy are the CLI
- draconic build fetches missing locked deps unless --offline
- Resolve lands on ESM files inside the package

This is lookup, not the Learn chapter. See [CLI](cli.html) for the command names.
