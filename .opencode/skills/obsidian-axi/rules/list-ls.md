---
title: Recursive list, newest first
impact: MEDIUM
impactDescription: a flat ls hides kind folders
tags: [list, ls]
---

## Recursive list, newest first

`ls` lists notes. `-r` / `--recursive` walks folders. `--sort modified` puts newest first.

**Incorrect:** `obsidian-axi ls --vault docs` then guessing paths under `architecture/` and `guides/`.

**Correct:** `obsidian-axi ls -r --sort modified --vault docs`

Notes: This repo's kind folders live directly under `docs/` (`layout-vault` on the **docs** skill). Ignore `docs/99_scribble/` if it appears. `--fields path` cuts columns (`list-fields`).
