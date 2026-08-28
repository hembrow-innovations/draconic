---
title: modules
section: learn
status: not-yet
---

# modules

A Program may be one file or an ECMAScript Module graph. The Linker loads an entry path's ESM import graph, mangles bindings, and flattens to one Program. That step is not a bundler product; Frontend chooses parse versus link.

Packages resolve to ESM files inside a git-backed tree. Draconic does not replace ESM with a different module syntax.

Script versus Module policy lives in the Frontend. Callers do not wire stage crates by hand.

Continue to [native types](native-types.html).
