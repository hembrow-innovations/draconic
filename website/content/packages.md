---
title: packages
section: learn
status: shipped
---

# packages

Packages are git-backed. v1 does not require a central registry.

Identity is hybrid. Imports use a Go-like module path such as `github.com/org/pkg`. A `draconic.toml` manifest may map that path to a git URL when default URL derivation is wrong.

Versions are semver git tags. The lockfile `draconic.lock` pins commit OID and a content hash of the package tree. Resolve lands on ESM files inside the package. Draconic does not replace ESM. That is the join back to [modules](modules.html).

## Package root

A package is a git tree with a `draconic.toml` and an ESM entry. Save this as `index.drac`. It builds today:

```drac
let console = globalThis.console;

export function greet(name: string): string {
  return "hello " + name;
}

console.log(greet("from a package"));
```

Parse it, typecheck it, or run it. Default `draconic run` target is js:

```
draconic parse index.drac
draconic check index.drac
draconic run index.drac
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

## Flagship service

pkg-lib and pkg-consumer are the pair. Copy [Flagship service](https://github.com/hembrow-innovations/draconic/tree/main/examples/flagship-service) when you want typed HTTP, filesystem config, and a git dependency in one Program.

Lookup: [packages](reference-packages.html) and [CLI](cli.html).
