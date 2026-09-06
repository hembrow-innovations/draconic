---
title: Cut columns with --fields
impact: MEDIUM
impactDescription: default rows carry extra columns on purpose
tags: [list, fields]
---

## Cut columns with --fields

List rows carry several columns. `--fields path` drops about a quarter. Use it when you only need paths.

**Incorrect:** Asking the model to strip TOON columns by hand, or converting to JSON to project fields.

**Correct:** `obsidian-axi ls -r --fields path --vault docs`

Notes: The bigger saving on a single note is `read --metadata` (`read-metadata`). Keep default columns when you need mtime or tags in the same list.
