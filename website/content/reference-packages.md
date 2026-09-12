---
title: packages
section: reference
status: shipped
---

# packages

Packages are git-backed. v1 does not require a central registry. This page is lookup, not the Learn chapter. See [packages](packages.html) for the flow and [CLI](cli.html) for command names.

- Imports use a Go-like module path such as github.com/org/pkg
- draconic.toml may map that path to a git URL
- Versions are semver git tags
- draconic.lock pins commit OID and a content hash of the package tree
- Resolve lands on ESM files inside the package

## Package root

A package is a git tree with a `draconic.toml` and an ESM entry. Save this as `index.drac`. It builds today:

```drac
let console = globalThis.console;

export function greet(name: string): string {
  return "hello " + name;
}

console.log(greet("from a package"));
```

```
draconic parse index.drac
draconic check index.drac
```

The matching manifest names the module path:

```
module = "github.com/org/pkg"
```

## get

`draconic get` adds a git package. It writes the dependency into `draconic.toml`, fetches, and updates `draconic.lock`. There is no published central registry in v1. Use `--url` when the default `https://{module_path}.git` is wrong, including a local git checkout:

```
draconic get github.com/org/pkg@1.0.0
draconic get github.com/org/pkg@1.0.0 --url /path/to/pkg
```

## mod tidy

`draconic mod tidy` aligns the lockfile with the manifest, fetches missing packages, and prunes unused ones:

```
draconic mod tidy
```

`draconic build` fetches missing locked deps unless `--offline`.

A consumer imports named exports with the module path, not a relative `.drac` specifier:

```
import { greet } from "github.com/org/pkg";
```

Copy [pkg-lib](https://github.com/hembrow-innovations/draconic/tree/main/examples/pkg-lib) and [pkg-consumer](https://github.com/hembrow-innovations/draconic/tree/main/examples/pkg-consumer) when you want a package root plus a consumer in one tree.
