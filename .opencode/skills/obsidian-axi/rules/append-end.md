---
title: Append is the end of the file
impact: HIGH
impactDescription: append does not target a heading
tags: [append, mut]
---

## Append is the end of the file

`append` adds to the end of the note. A heading or frontmatter change is `patch`.

**Incorrect:** `obsidian-axi append note.md --content "- [ ] item"` expecting it under `## Tasks`.

**Correct:** `obsidian-axi patch note.md --target-type heading --target Tasks --content "- [ ] item" --vault docs`

Notes: stdin works: `echo "- piped" | obsidian-axi append Inbox/log.md --vault docs`. Retry is safe when the text is already at the end (`mut-idempotent`).
