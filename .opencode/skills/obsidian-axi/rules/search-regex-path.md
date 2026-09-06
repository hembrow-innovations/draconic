---
title: Regex plus path glob
impact: HIGH
impactDescription: unanchored regex without --path scans the whole vault
tags: [search, regex]
---

## Regex plus path glob

`--regex` treats the query as a regular expression. Pair it with `--path` so the scan stays inside one kind folder.

**Incorrect:** `obsidian-axi search "^## " --regex --vault docs`

**Correct:** `obsidian-axi search "^## " --regex --path "architecture/**" --vault docs`

Notes: `--path` is a glob, quoted. Combine with `--tag` or `--frontmatter` when those already split the set. This CLI is not Dataview (`layer-unsupported`).
