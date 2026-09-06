---
title: rm goes to .trash
impact: HIGH
impactDescription: --permanent is unrecoverable
tags: [rm, trash]
---

## rm goes to .trash

`rm` moves the note to `.trash` so it can be recovered. `--permanent` deletes it. A missing note is a no-op, exit 0.

**Incorrect:** `obsidian-axi rm docs/guides/guides-heio-stack.md --permanent --vault docs` as the default delete.

**Correct:** `obsidian-axi rm Inbox/draft.md --vault docs`

Notes: After a rename or delete, fix leftover wikilinks via **docs** `link-wikilinks`. Never trash `.heio/` notes with this CLI (`layer-heio-boundary`).
