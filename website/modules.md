---
title: modules
section: learn
status: shipped
---

# modules

A Program may be one file or an ECMAScript Module graph. Draconic does not replace ESM with a different module syntax. You write `import` and `export` as in JavaScript modules.

Script versus Module is a Frontend policy. A single file with no `import` or `export` is still a Program. Add either and that file is a module: the toolchain loads the ESM graph from the entry, mangles bindings so names do not collide, and flattens the graph to one Program. That step is linking, not a bundler product. The Frontend chooses parse versus link; callers do not wire those stages by hand.

Save this as `greet.drac`. It builds today:

```drac
let console = globalThis.console;

export function greet(name: string): string {
  return "hello " + name;
}

console.log(greet("from Draconic"));
```

Parse it, typecheck it, or run it. Default `draconic run` target is js:

```
draconic parse greet.drac
draconic check greet.drac
draconic run greet.drac
```

A second file imports named exports with a static relative specifier that ends in `.drac`. There is no extensionless import and no bare specifier on this page. Save `main.drac` next to `greet.drac`:

```
import { greet } from "./greet.drac";
```

Build the entry. The Frontend links the graph. Relative specifiers must exist on disk.

Packages resolve to ESM files inside a git-backed tree. A bare specifier such as `github.com/org/pkg` is the join to [packages](packages.html): a module path is not a new language, it is how an import finds a file.

Continue to [native types](native-types.html).
