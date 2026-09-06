---
title: modules
section: learn
status: not-yet
---

# modules

A Program may be one file or an ECMAScript Module graph. Draconic does not replace ESM with a different module syntax. You write `import` and `export` as in JavaScript modules.

When the entry is a module, the toolchain loads that path's ESM import graph, mangles bindings so names do not collide, and flattens the graph to one Program. That step is linking, not a bundler product. The Frontend chooses parse versus link; callers do not wire those stages by hand.

Script versus Module is a Frontend policy. A single file with no module graph is still a Program.

Packages resolve to ESM files inside a git-backed tree. That is the join to [packages](packages.html): a module path is not a new language, it is how an import finds a file.

Continue to [native types](native-types.html).
