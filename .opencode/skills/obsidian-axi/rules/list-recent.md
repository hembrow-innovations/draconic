---
title: Recent notes by mtime
impact: MEDIUM
impactDescription: --opened is workspace, not disk
tags: [list, recent]
---

## Recent notes by mtime

`recent --days N` is filesystem mtime. `recent --opened` reads the saved Obsidian workspace, which is only written on layout change.

**Incorrect:** `obsidian-axi recent --opened --vault docs` as “what I edited in git today”.

**Correct:** `obsidian-axi recent --days 7 --vault docs`

Notes: This checkout may have no workspace file. Prefer `--days` here. Dashboard no-args already shows recent (`disc-dashboard-first`).
