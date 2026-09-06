---
title: Dataview and Templater are UNSUPPORTED
impact: LOW
impactDescription: those features need a running Obsidian
tags: [layer, unsupported]
---

## Dataview and Templater are UNSUPPORTED

Dataview, Templater, semantic search, and the command palette need a running Obsidian. This CLI returns `UNSUPPORTED` rather than guessing.

**Incorrect:** Asking `search` to evaluate a Dataview `LIST FROM` block, or inventing a Templater expansion.

**Correct:** `obsidian-axi search --frontmatter status=active --vault docs` for field filters. Frontmatter and wikilinks are on disk; Dataview queries are not.

Notes: Docs notes may still *contain* Dataview blocks for humans. Do not execute them here. Tag and frontmatter filters cover the usual agent need (`search-narrow`, `graph-tags`).
