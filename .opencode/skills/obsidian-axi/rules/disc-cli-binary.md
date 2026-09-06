---
title: Resolve the filesystem CLI
impact: CRITICAL
impactDescription: the unscoped package needs a running Obsidian app
tags: [disc, cli]
---

## Resolve the filesystem CLI

This pack runs `@andershoffmann/obsidian-axi`. It reads the vault on disk. The unscoped `obsidian-axi` package is a different wrapper.

```sh
npx -y @andershoffmann/obsidian-axi --help
```

**Incorrect:** `npx obsidian-axi search foo` or `obsidian search foo`.

**Correct:** `npx -y @andershoffmann/obsidian-axi search foo --vault docs`. If `obsidian-axi --help` already works (global install of the scoped package), use that binary.

Notes: Do not run `setup hooks` unless the human asked. Follow-up lines that start with `obsidian-axi` still go through the binary you resolved here.
