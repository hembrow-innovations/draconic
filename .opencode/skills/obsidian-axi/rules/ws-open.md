---
title: open asks the Obsidian app
impact: MEDIUM
impactDescription: the rest of this CLI does not need the app
tags: [ws, open]
---

## open asks the Obsidian app

`open <path>` tells Obsidian to jump to a note. Everything else in this CLI is filesystem. `open` is the exception.

**Incorrect:** `obsidian-axi open architecture-heio-stack.md --vault docs` as the way to read a note.

**Correct:** `obsidian-axi read architecture-heio-stack.md --vault docs`. Use `open` only when the human asked to see it in the app.

Notes: If Obsidian is not running, `open` fails. Reading, search, and patch do not care.
