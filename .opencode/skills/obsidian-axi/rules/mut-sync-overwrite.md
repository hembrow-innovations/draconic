---
title: Unsaved app buffers win
impact: HIGH
impactDescription: Obsidian can overwrite a CLI write on next save
tags: [mut, sync]
---

## Unsaved app buffers win

Writes go via temp file then rename. Obsidian reloads outside edits. An unsaved buffer in the app can still overwrite that write on the next save.

**Incorrect:** Patching a note the human has open and dirty, then assuming disk is source of truth.

**Correct:** When the app may be open, run `obsidian-axi active --vault docs` first (`ws-active`). Ask the human to save or close if that note is the target. Then patch.

Notes: This checkout often has no workspace file. If `active` errors, proceed on disk and mention the risk. `--permanent` delete is still a delete (`rm-trash`).
