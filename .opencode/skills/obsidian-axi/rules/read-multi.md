---
title: Read several paths in one call
impact: HIGH
impactDescription: one process, one vault resolve, fewer round trips
tags: [read, multi]
---

## Read several paths in one call

`read` takes more than one path. Independent notes do not need serial calls.

**Incorrect:**

```sh
obsidian-axi read a.md --vault docs
obsidian-axi read b.md --vault docs
```

**Correct:** `obsidian-axi read a.md b.md --vault docs`

Notes: Still truncated per body. Add `--metadata` when you only need fronts. Add `--full` only when every body must be complete.
