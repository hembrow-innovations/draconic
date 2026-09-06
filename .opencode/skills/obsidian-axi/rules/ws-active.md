---
title: active is last saved layout
impact: MEDIUM
impactDescription: workspace.json is not the live tab
tags: [ws, active]
---

## active is last saved layout

`active` reads `.obsidian/workspace.json`. Obsidian writes that file on layout change, not every tab switch. Output includes how old it is.

**Incorrect:** Treating `active` as “the note on screen right now”.

**Correct:** Read the age in the TOON. If the file is missing (this checkout), skip `active` and use `recent --days` (`list-recent`).

Notes: Check `active` before a write only when the human may have the app open (`mut-sync-overwrite`).
