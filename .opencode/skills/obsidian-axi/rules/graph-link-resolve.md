---
title: Wikilink resolution order
impact: HIGH
impactDescription: a unique filename is enough; a wrong folder is not a miss
tags: [graph, wikilink]
---

## Wikilink resolution order

A link resolves in order: exact path, then relative to the linking note, then matching filename anywhere, then frontmatter aliases. Embeds (`![[Pasted image.png]]`) count as resolved.

**Incorrect:** Calling `[[guides-heio-stack]]` broken because the file lives under `docs/guides/`.

**Correct:** `obsidian-axi links --broken --vault docs` and treat only true misses. This repo still writes `[[note-name]]`, not relative `.md` paths.

Notes: After `mv`, search for the old path and fix leftovers before commit (docs `link-wikilinks`). Orphans are notes with no incoming links (`graph-links`).
